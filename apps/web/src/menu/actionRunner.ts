import { useProjectStore } from "../store/projectStore";
import { useUIStore } from "../store/uiStore";
import { useTransportStore } from "../store/transportStore";
import { useMetronomeStore } from "../store/metronomeStore";
import { useHistoryStore } from "../store/historyStore";
import { transport } from "../engine/Transport";
import { clipScheduler } from "../engine/ClipScheduler";
import { DeleteTrackCommand, DeleteClipsCommand, DuplicateClipsCommand } from "../commands";

export function runAction(actionId: string) {
  const { toggleCommandPalette } = useUIStore.getState();

  // Close the palette if an action is run
  if (useUIStore.getState().commandPaletteOpen) {
    useUIStore.getState().setCommandPaletteOpen(false);
  }

  const projectStore = useProjectStore.getState();
  const uiStore = useUIStore.getState();
  const transportStore = useTransportStore.getState();
  const metronomeStore = useMetronomeStore.getState();

  switch (actionId) {
    // Tools
    case "tools:command-palette":
    case "tools:quick-search":
      toggleCommandPalette();
      break;

    case "command:close":
      useUIStore.getState().setCommandPaletteOpen(false);
      break;

    // Transport
    case "transport:play-pause":
      if (transportStore.isPlaying) {
        transport.pause();
        clipScheduler.cancelAll();
        transportStore.setIsPlaying(false);
      } else {
        void transport.play().then(() => {
          transportStore.setIsPlaying(true);
        });
      }
      break;

    case "transport:stop":
      transport.stop();
      transportStore.setIsPlaying(false);
      break;

    case "transport:go-to-start":
      transport.seek(0);
      if (transportStore.isPlaying) {
        clipScheduler.cancelAll();
        clipScheduler.schedule(projectStore.project.tracks);
      }
      break;

    case "transport:toggle-loop":
      uiStore.toggleLoop();
      break;

    case "transport:toggle-metronome":
      metronomeStore.toggle();
      break;

    case "transport:toggle-count-in":
      metronomeStore.toggleCountIn();
      break;

    // Edit
    case "edit:undo":
      useHistoryStore.getState().undo();
      break;

    case "edit:redo":
      useHistoryStore.getState().redo();
      break;

    case "edit:delete": {
      const { selectedClipIds, selectedTrackId, focusedPanel } = uiStore;
      if (focusedPanel === "timeline" && selectedClipIds.length > 0) {
        useHistoryStore.getState().execute(new DeleteClipsCommand(selectedClipIds));
        uiStore.setSelectedClipIds([]);
      } else if (selectedTrackId) {
        useHistoryStore.getState().execute(new DeleteTrackCommand(selectedTrackId));
        uiStore.setSelectedTrackId(null);
        uiStore.setSelectedMixerTrackId(null);
      }
      break;
    }

    case "edit:delete-track": {
      const { selectedTrackId } = uiStore;
      if (selectedTrackId) {
        useHistoryStore.getState().execute(new DeleteTrackCommand(selectedTrackId));
        uiStore.setSelectedTrackId(null);
        uiStore.setSelectedMixerTrackId(null);
      }
      break;
    }

    case "edit:duplicate": {
      const { selectedClipIds } = uiStore;
      if (selectedClipIds.length > 0) {
        useHistoryStore.getState().execute(new DuplicateClipsCommand(selectedClipIds));
      }
      break;
    }

    case "edit:deselect-all":
      uiStore.setSelectedClipIds([]);
      uiStore.setSelectedTrackId(null);
      break;

    case "timeline:toggle-snap":
      uiStore.toggleSnapToGrid();
      break;

    // View
    case "view:toggle-mixer": // assuming this might be an action, or mapping directly
      uiStore.toggleMixer();
      break;
    
    case "view:toggle-inspector":
      uiStore.toggleInspector();
      break;

    // Project
    case "project:save":
      projectStore.saveLocal();
      break;

    case "noop":
      break;

    default:
      console.warn(`[ActionRunner] Unhandled action: ${actionId}`);
  }
}
