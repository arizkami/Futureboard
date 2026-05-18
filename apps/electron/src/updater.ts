import { app, dialog } from "electron";
import { autoUpdater, type UpdateInfo } from "electron-updater";

export function initAutoUpdater(): void {
  if (!app.isPackaged) return;

  autoUpdater.autoDownload = true;
  autoUpdater.autoInstallOnAppQuit = true;

  autoUpdater.on("update-available", (info: UpdateInfo) => {
    console.log(`[AutoUpdater] Update available: ${info.version}`);
  });

  autoUpdater.on("update-not-available", (info: UpdateInfo) => {
    console.log(`[AutoUpdater] Up to date: ${info.version}`);
  });

  autoUpdater.on("update-downloaded", (info: UpdateInfo) => {
    dialog
      .showMessageBox({
        type: "info",
        title: "Update Ready",
        message: `Futureboard Studio ${info.version} is ready to install.`,
        detail:
          "Restart now to apply the update, or it will be installed next time you launch.",
        buttons: ["Restart Now", "Later"],
        defaultId: 0,
        cancelId: 1,
      })
      .then(({ response }) => {
        if (response === 0) autoUpdater.quitAndInstall(false, true);
      })
      .catch(() => {});
  });

  autoUpdater.on("error", (err: Error) => {
    console.error("[AutoUpdater] Error:", err?.message ?? String(err));
  });

  // Delay first check slightly so it doesn't compete with cold-start I/O.
  setTimeout(() => {
    autoUpdater.checkForUpdatesAndNotify().catch((err: unknown) => {
      console.warn(
        "[AutoUpdater] checkForUpdates failed:",
        err instanceof Error ? err.message : String(err),
      );
    });
  }, 10_000);
}
