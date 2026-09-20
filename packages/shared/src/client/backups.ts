// The copies of the shop file the server keeps: listing them, taking one,
// and restoring from one.

import type { BackupDto } from "../generated/BackupDto";
import type { BackupsDto } from "../generated/BackupsDto";
import type { RestoreDto } from "../generated/RestoreDto";
import { narrow } from "../client-response";
import type { Transport } from "../client";
import { backupSchema, backupsSchema, restoreSchema } from "../schemas/settings";

export function backupsClient({ send }: Transport) {
  return {
    /** The copies of the shop file the server keeps, newest first: the daily
     * ones, the copies taken on the way into a restore, and the copies taken
     * on the way into an update. The three are kept under different rules
     * and so travel in their own lists. */
    async listBackups(): Promise<BackupsDto> {
      return narrow(await send("/backups"), backupsSchema, "backup list");
    },

    /** One more copy, taken now. The server names it and prunes the folder. */
    async createBackup(): Promise<BackupDto> {
      return narrow(await send("/backups", { method: "POST" }), backupSchema, "backup");
    },

    /** Puts the shop file back from a copy. The name is the server's own, and
     * it is encoded rather than spliced, so a name that somehow carried a
     * separator reaches the server as one segment and is refused there. */
    async restoreBackup(name: string): Promise<RestoreDto> {
      const body = await send(`/backups/${encodeURIComponent(name)}/restore`, {
        method: "POST",
      });
      return narrow(body, restoreSchema, "restore answer");
    },
  };
}
