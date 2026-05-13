import type { FileId, WaveformPeaks } from "../types/daw";

export function generatePeaks(
  fileId: FileId,
  audioBuffer: AudioBuffer,
  onPeaks: (fileId: FileId, peaks: WaveformPeaks) => void
): void {
  const channelData: Float32Array[] = [];
  for (let c = 0; c < audioBuffer.numberOfChannels; c++) {
    channelData.push(audioBuffer.getChannelData(c).slice());
  }

  const worker = new Worker(
    new URL("../workers/waveformWorker.ts", import.meta.url),
    { type: "module" }
  );
  worker.postMessage(
    { fileId, channelData, samplesPerPeak: 256 },
    channelData.map((c) => c.buffer)
  );
  worker.onmessage = (e: MessageEvent<{ fileId: FileId; peaks: WaveformPeaks }>) => {
    onPeaks(e.data.fileId, e.data.peaks);
    worker.terminate();
  };
}
