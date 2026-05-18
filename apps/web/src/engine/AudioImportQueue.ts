import type { DawFile, DawProjectAsset, FileId, WaveformPeaks } from "../types/daw";
import { audioAssetManager, type ImportedAudioAsset } from "./AudioAssetManager";
import { waveformCache, buildCacheKey, entryPeaksAsInt16, SAMPLES_PER_PEAK, WAVEFORM_CACHE_VERSION } from "./waveformCache";
import { platform } from "../platform";
import { useProjectStore } from "../store/projectStore";
import { addFileToTimeline, isImportableAudioFile, readWavMetadata } from "../utils/importAudioToProject";
import { showToast } from "../components/ui/Toast";

export const IMPORT_LIMITS = {
  copyConcurrency: 2,
  metadataConcurrency: 2,
  peakConcurrency: 1,
  decodeConcurrency: 1,
};

export type AudioImportQueueState =
  | "pending"
  | "copying"
  | "indexing"
  | "generating-peaks"
  | "ready"
  | "failed";

export type AudioImportJob = {
  id: string;
  fileId: FileId;
  fileName: string;
  size: number;
  state: AudioImportQueueState;
  error?: string;
  sourcePath?: string;
  createdAt: number;
  updatedAt: number;
};

type QueueSource = {
  file?: File;
  sourcePath?: string;
  name: string;
  size: number;
  lastModified?: number;
  mimeType?: string;
};

type TimelineTarget = {
  startTime?: number;
  trackId?: string;
};

type EnqueueOptions = TimelineTarget & {
  fileId?: FileId;
};

type Listener = () => void;

type WorkerMessage =
  | { type: "progress"; fileId: FileId; progress: number; samplesPerPeak: number }
  | { type: "peaks"; fileId: FileId; peaks: WaveformPeaks }
  | { type: "completed"; fileId: FileId }
  | { type: "error"; fileId: FileId; message: string };

class AudioImportQueue {
  private jobs = new Map<string, AudioImportJob>();
  private sources = new Map<string, QueueSource>();
  private targets = new Map<string, TimelineTarget>();
  private queue: string[] = [];
  private activeCopies = 0;
  private activePeaks = 0;
  private listeners = new Set<Listener>();
  private decodedBuffers = new Map<FileId, AudioBuffer>();
  private decodeQueue: Array<{
    file: DawFile;
    resolve: (buffer: AudioBuffer | null) => void;
  }> = [];
  private activeDecodes = 0;
  private peakWorkers = new Map<FileId, Worker>();
  private sourceTotalBytes = 0;

  subscribe(listener: Listener): () => void {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }

  getJobs(): AudioImportJob[] {
    return [...this.jobs.values()].sort((a, b) => a.createdAt - b.createdAt);
  }

  getJob(fileId: FileId): AudioImportJob | undefined {
    return this.jobs.get(fileId);
  }

  getDebugStats() {
    let peakBytes = 0;
    for (const peaks of useProjectStore.getState().peakCache.values()) {
      peakBytes += peaks.peaks.byteLength;
    }
    let decodedBytes = 0;
    for (const buffer of this.decodedBuffers.values()) {
      decodedBytes += buffer.length * buffer.numberOfChannels * 4;
    }
    return {
      sourceTotalMB: this.sourceTotalBytes / 1024 / 1024,
      decodedBuffersCount: this.decodedBuffers.size,
      decodedBuffersMB: decodedBytes / 1024 / 1024,
      peakCacheMB: peakBytes / 1024 / 1024,
      importQueueLength: this.queue.length,
      activeJobs: this.activeCopies + this.activePeaks + this.activeDecodes,
    };
  }

  enqueueFile(file: File, options: EnqueueOptions): DawFile | null {
    if (!isImportableAudioFile(file)) return null;
    const sourcePath = platform.fileSystem.getNativePathForFile(file) ?? undefined;
    return this.enqueueSource({
      file,
      sourcePath,
      name: file.name,
      size: file.size,
      lastModified: file.lastModified,
      mimeType: file.type,
    }, options);
  }

  async enqueueNativePath(path: string, options: EnqueueOptions): Promise<DawFile | null> {
    const stat = await platform.fileSystem.statAudioFile(path).catch(() => null);
    if (!stat) return null;
    return this.enqueueSource({
      sourcePath: path,
      name: stat.name,
      size: stat.size,
      lastModified: stat.lastModified,
      mimeType: stat.mimeType,
    }, options);
  }

  enqueueFiles(files: File[], options: TimelineTarget): DawFile[] {
    const imported: DawFile[] = [];
    let offset = 0;
    for (const file of files) {
      const placeholder = this.enqueueFile(file, { ...options, startTime: (options.startTime ?? 0) + offset });
      if (!placeholder) continue;
      imported.push(placeholder);
      offset += 0.05;
    }
    if (imported.length > 1) showToast("Importing audio...");
    return imported;
  }

  evictDecodedBuffer(fileId: FileId): void {
    this.decodedBuffers.delete(fileId);
  }

  async ensureDecodedBuffer(file: DawFile): Promise<AudioBuffer | null> {
    const cached = this.decodedBuffers.get(file.id);
    if (cached) return cached;
    return new Promise((resolve) => {
      this.decodeQueue.push({ file, resolve });
      this.pumpDecodeQueue();
    });
  }

  private enqueueSource(source: QueueSource, options: EnqueueOptions): DawFile {
    const fileId = options.fileId ?? crypto.randomUUID();
    const now = Date.now();
    const manifest = audioAssetManager.createAssetManifest(fileId, source);
    const placeholder: DawFile = {
      id: fileId,
      name: source.name,
      mimeType: source.mimeType || mimeFromName(source.name),
      size: source.size,
      lastModified: source.lastModified,
      originalFileName: source.name,
      duration: 1,
      sampleRate: 48000,
      channels: 1,
      ...manifest,
      localObjectUrl: undefined,
    };
    const store = useProjectStore.getState();
    store.addFile(placeholder);
    store.setWaveformStatus(fileId, "pending");
    store.setWaveformProgress(fileId, 0);
    if (options.startTime != null) {
      addFileToTimeline(placeholder, options.startTime, options.trackId);
    }

    this.jobs.set(fileId, {
      id: crypto.randomUUID(),
      fileId,
      fileName: source.name,
      size: source.size,
      sourcePath: source.sourcePath,
      state: "pending",
      createdAt: now,
      updatedAt: now,
    });
    this.sources.set(fileId, source);
    this.targets.set(fileId, options);
    this.queue.push(fileId);
    this.sourceTotalBytes += source.size;
    this.emit();
    this.pumpImportQueue();
    return placeholder;
  }

  private pumpImportQueue(): void {
    while (this.activeCopies < IMPORT_LIMITS.copyConcurrency && this.queue.length > 0) {
      const fileId = this.queue.shift();
      if (!fileId) return;
      this.activeCopies++;
      void this.processJob(fileId)
        .catch((error) => this.failJob(fileId, error))
        .finally(() => {
          this.activeCopies--;
          this.sources.delete(fileId);
          this.targets.delete(fileId);
          this.pumpImportQueue();
          this.emit();
        });
    }
  }

  private async processJob(fileId: FileId): Promise<void> {
    const source = this.sources.get(fileId);
    if (!source) return;
    this.setJobState(fileId, "copying");
    const savedManifest = await audioAssetManager.saveImportedAudioAsset(fileId, source);

    this.setJobState(fileId, "indexing");
    const meta = await this.readMetadata(source, savedManifest);
    const current = useProjectStore.getState().project.files.find((f) => f.id === fileId);
    const duration = meta?.duration ?? current?.duration ?? 1;
    const updates: Partial<DawFile> = {
      ...savedManifest,
      duration,
      sampleRate: meta?.sampleRate ?? current?.sampleRate ?? 48000,
      channels: meta?.channels ?? current?.channels ?? 1,
      name: savedManifest.name ?? current?.name ?? source.name,
      size: savedManifest.size ?? current?.size ?? source.size,
      lastModified: savedManifest.lastModified ?? current?.lastModified ?? source.lastModified,
      mimeType: source.mimeType || current?.mimeType || mimeFromName(source.name),
      localObjectUrl: undefined,
    };
    useProjectStore.getState().updateFile(fileId, updates);
    this.updateClipsForFile(fileId, duration);
    this.registerAsset(fileId, source, savedManifest, meta);

    const cached = await audioAssetManager.loadCachedWaveform({ ...(current ?? {}), id: fileId, ...updates } as DawFile);
    if (cached) {
      useProjectStore.getState().setPeaks(fileId, cached);
      this.setJobState(fileId, "ready");
      return;
    }

    await this.queuePeakJob(fileId, source, savedManifest, meta?.duration ?? duration);
  }

  private async readMetadata(source: QueueSource, manifest: ImportedAudioAsset) {
    if (source.file) return readWavMetadata(source.file);
    void manifest;
    return null;
  }

  private registerAsset(fileId: FileId, source: QueueSource, manifest: ImportedAudioAsset, meta: Awaited<ReturnType<typeof readWavMetadata>>) {
    if (manifest.storageProvider !== "project-folder" || !manifest.relativePath) return;
    const now = new Date().toISOString();
    const asset: DawProjectAsset = {
      id: fileId,
      type: "audio",
      name: manifest.name ?? source.name,
      originalName: source.name,
      relativePath: manifest.relativePath,
      size: manifest.size ?? source.size,
      durationSeconds: meta?.duration,
      sampleRate: meta?.sampleRate,
      channels: meta?.channels,
      mimeType: source.mimeType || mimeFromName(source.name),
      createdAt: now,
      updatedAt: now,
    };
    useProjectStore.getState().addAsset(asset);
  }

  private updateClipsForFile(fileId: FileId, duration: number): void {
    for (const track of useProjectStore.getState().project.tracks) {
      for (const clip of track.clips) {
        if (clip.fileId === fileId) {
          useProjectStore.getState().updateClip(clip.id, { duration, assetId: fileId });
        }
      }
    }
  }

  private queuePeakJob(fileId: FileId, source: QueueSource, manifest: ImportedAudioAsset, duration: number): Promise<void> {
    return new Promise((resolve, reject) => {
      const start = () => {
        this.activePeaks++;
        this.setJobState(fileId, "generating-peaks");
        useProjectStore.getState().setWaveformStatus(fileId, "generating-peaks");
        this.runPeakWorker(fileId, source, manifest, duration)
          .then(resolve)
          .catch(reject)
          .finally(() => {
            this.activePeaks--;
            this.pumpDeferredPeakJobs();
          });
      };
      this.deferredPeakJobs.push(start);
      this.pumpDeferredPeakJobs();
    });
  }

  private deferredPeakJobs: Array<() => void> = [];

  private pumpDeferredPeakJobs(): void {
    while (this.activePeaks < IMPORT_LIMITS.peakConcurrency && this.deferredPeakJobs.length > 0) {
      const next = this.deferredPeakJobs.shift();
      next?.();
    }
  }

  private async runPeakWorker(fileId: FileId, source: QueueSource, manifest: ImportedAudioAsset, duration: number): Promise<void> {
    const peakSource = source.file ?? null;
    if (!peakSource) {
      const nativePath = manifest.cacheKey ?? manifest.storageKey ?? source.sourcePath;
      const nativePeaks = nativePath
        ? await platform.fileSystem.generateWavPeaks(nativePath, fileId, SAMPLES_PER_PEAK)
        : null;
      if (!nativePeaks) {
        useProjectStore.getState().setWaveformStatus(fileId, "idle");
        this.setJobState(fileId, "ready");
        return;
      }

      const peaks: WaveformPeaks = {
        fileId,
        samplesPerPeak: nativePeaks.samplesPerPeak,
        channelCount: nativePeaks.channelCount,
        peakCount: nativePeaks.peakCount,
        peaks: new Int16Array(nativePeaks.peaks),
        sampleRate: nativePeaks.sampleRate,
        duration: nativePeaks.duration,
        version: WAVEFORM_CACHE_VERSION,
      };
      useProjectStore.getState().updateFile(fileId, {
        duration: peaks.duration,
        sampleRate: peaks.sampleRate,
        channels: peaks.channelCount,
      });
      this.updateClipsForFile(fileId, peaks.duration ?? duration);
      useProjectStore.getState().setPeaks(fileId, peaks);
      await waveformCache.set(buildCacheKey(fileId, peaks.samplesPerPeak), {
        version: WAVEFORM_CACHE_VERSION,
        fileId,
        sampleRate: peaks.sampleRate ?? 48000,
        channelCount: peaks.channelCount,
        duration: peaks.duration ?? duration,
        samplesPerPeak: peaks.samplesPerPeak,
        peakCount: peaks.peakCount ?? 0,
        createdAt: Date.now(),
        peaks: peaks.peaks,
      }).catch((error) => console.warn("[WaveformCache] set failed:", error));
      this.setJobState(fileId, "ready");
      return;
    }
    await new Promise<void>((resolve, reject) => {
      const worker = new Worker(new URL("../workers/waveformWorker.ts", import.meta.url), { type: "module" });
      this.peakWorkers.set(fileId, worker);
      worker.onmessage = (e: MessageEvent<WorkerMessage>) => {
        if (e.data.type === "progress") {
          useProjectStore.getState().setWaveformProgress(fileId, e.data.progress);
          return;
        }
        if (e.data.type === "peaks") {
          useProjectStore.getState().updateFile(fileId, {
            duration: e.data.peaks.duration,
            sampleRate: e.data.peaks.sampleRate,
            channels: e.data.peaks.channelCount,
          });
          this.updateClipsForFile(fileId, e.data.peaks.duration ?? duration);
          useProjectStore.getState().setPeaks(fileId, e.data.peaks);
          waveformCache.set(buildCacheKey(fileId, e.data.peaks.samplesPerPeak), {
            version: WAVEFORM_CACHE_VERSION,
            fileId,
            sampleRate: e.data.peaks.sampleRate ?? 48000,
            channelCount: e.data.peaks.channelCount,
            duration: e.data.peaks.duration ?? duration,
            samplesPerPeak: e.data.peaks.samplesPerPeak,
            peakCount: e.data.peaks.peakCount ?? 0,
            createdAt: Date.now(),
            peaks: entryPeaksAsInt16({
              version: WAVEFORM_CACHE_VERSION,
              fileId,
              sampleRate: e.data.peaks.sampleRate ?? 48000,
              channelCount: e.data.peaks.channelCount,
              duration: e.data.peaks.duration ?? duration,
              samplesPerPeak: e.data.peaks.samplesPerPeak,
              peakCount: e.data.peaks.peakCount ?? 0,
              createdAt: Date.now(),
              peaks: e.data.peaks.peaks,
            }),
          }).catch((error) => console.warn("[WaveformCache] set failed:", error));
          return;
        }
        if (e.data.type === "completed") {
          this.setJobState(fileId, "ready");
          this.peakWorkers.delete(fileId);
          worker.terminate();
          resolve();
          return;
        }
        if (e.data.type === "error") {
          this.peakWorkers.delete(fileId);
          worker.terminate();
          reject(new Error(e.data.message));
        }
      };
      worker.onerror = () => {
        this.peakWorkers.delete(fileId);
        worker.terminate();
        reject(new Error("Waveform worker failed"));
      };
      worker.postMessage({
        fileId,
        source: peakSource,
        sampleRate: undefined,
        duration,
        samplesPerPeakList: [SAMPLES_PER_PEAK],
      });
    });
  }

  private pumpDecodeQueue(): void {
    while (this.activeDecodes < IMPORT_LIMITS.decodeConcurrency && this.decodeQueue.length > 0) {
      const job = this.decodeQueue.shift();
      if (!job) return;
      this.activeDecodes++;
      void this.decodeFile(job.file)
        .then(job.resolve)
        .catch((error) => {
          console.warn("[AudioImportQueue] lazy decode failed:", error);
          job.resolve(null);
        })
        .finally(() => {
          this.activeDecodes--;
          this.pumpDecodeQueue();
          this.emit();
        });
    }
  }

  private async decodeFile(file: DawFile): Promise<AudioBuffer | null> {
    const AudioContextCtor = window.AudioContext || (window as unknown as { webkitAudioContext?: typeof AudioContext }).webkitAudioContext;
    if (!AudioContextCtor) return null;
    const context = new AudioContextCtor();
    let sourceFile: File | null = null;
    if (file.storageProvider === "file-handle" || file.storageProvider === "project-folder") {
      const path = file.cacheKey ?? file.storageKey;
      if (path) sourceFile = await platform.fileSystem.readAudioFile(path).catch(() => null);
    }
    if (!sourceFile) return null;
    const buffer = await sourceFile.arrayBuffer();
    const decoded = await context.decodeAudioData(buffer);
    this.decodedBuffers.set(file.id, decoded);
    void context.close().catch(() => undefined);
    return decoded;
  }

  private setJobState(fileId: FileId, state: AudioImportQueueState): void {
    const job = this.jobs.get(fileId);
    if (job) {
      job.state = state;
      job.updatedAt = Date.now();
    }
    const waveformState = state === "failed" ? "error" : state === "ready" ? "ready" : state;
    useProjectStore.getState().setWaveformStatus(fileId, waveformState);
    this.emit();
  }

  private failJob(fileId: FileId, error: unknown): void {
    const job = this.jobs.get(fileId);
    if (job) {
      job.state = "failed";
      job.error = error instanceof Error ? error.message : String(error);
      job.updatedAt = Date.now();
    }
    useProjectStore.getState().setWaveformStatus(fileId, "error");
    showToast(`Could not import "${job?.fileName ?? "audio"}"`, true);
    this.emit();
  }

  private emit(): void {
    for (const listener of this.listeners) listener();
  }
}

function mimeFromName(name: string): string {
  const lower = name.toLowerCase();
  if (lower.endsWith(".wav")) return "audio/wav";
  if (lower.endsWith(".mp3")) return "audio/mpeg";
  return "audio/*";
}

export const audioImportQueue = new AudioImportQueue();
