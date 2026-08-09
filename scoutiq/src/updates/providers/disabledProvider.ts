import { readAppVersion } from "../version";
import type { UpdateCheckResult, UpdateProvider } from "../types";

/**
 * Active while automatic updates are not enabled.
 * Keeps Settings UX functional without contacting an update server.
 */
export const disabledUpdateProvider: UpdateProvider = {
  id: "disabled",

  async getCurrentVersion() {
    return readAppVersion();
  },

  async checkForUpdates(): Promise<UpdateCheckResult> {
    const currentVersion = await readAppVersion();

    // Simulate a short check so the UI status transition feels intentional.
    await new Promise((resolve) => {
      window.setTimeout(resolve, 450);
    });

    return {
      status: "disabled",
      currentVersion,
      messageKey: "settings.updates.status.disabled",
    };
  },
};
