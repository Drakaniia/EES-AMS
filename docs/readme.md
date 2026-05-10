To answer your questions about the database and storage:

1. Is rusqlite being used?
   Currently, it is not being used by the pages I just updated.
   While rusqlite is installed in your Rust backend and there is code in src-tauri to handle it, the frontend pages (Settings, Students, etc.) are currently importing their data logic from src/lib/db.ts, which uses IndexedDB (browser storage).

   Essentially, you have a "dual" setup right now:
   - Rust Backend: Has a SQLite implementation ready, but it's mostly idle.
   - Frontend: Handles all the data itself using IndexedDB.

2. If installed as an .exe, where will it store?
   If we switch to the Rust SQLite backend, Tauri follows standard OS conventions for data storage. It does not store data in C:\Program Files\YourApp (which requires Admin rights and is for code, not data).

   On Windows, the .db file would be stored in:
   `C:\Users\<YourName>\AppData\Roaming\<app-name>\attendance.db`

   This is the "correct" way for Windows apps to store persistent data because it is unique to the user and doesn't require special permissions.

3. What's better: IndexedDB (db.ts) or Rust SQLite?
   For a dedicated Desktop App, Rust SQLite is better. Here's why:

   | Feature     | IndexedDB (db.ts)                                            | Rust SQLite                                             |
   | ----------- | ------------------------------------------------------------ | ------------------------------------------------------- |
   | Reliability | Good, but can be cleared if the user "resets" browser cache. | Extremely robust. Standard for professional apps.       |
   | Performance | Slower for very large datasets (10k+ records).               | Very fast. Can handle millions of rows.                 |
   | Access      | Only accessible by the app window.                           | Accessible by the app window AND the background server. |
   | Backups     | Harder to find the raw file on disk.                         | Just a single .db file you can copy/paste.              |

## My Recommendation

Since you have a desktop app and already have rusqlite in your dependencies, we should ideally migrate the logic from db.ts to the Rust backend. This would make the app "true" desktop software.

However, I just implemented the "Multi-Class" feature in the db.ts (IndexedDB) side. The Rust backend doesn't know about "Classes" yet.

Would you like me to migrate the entire system (including the new Classes feature) to the Rust SQLite backend now? This would involve:

1. Updating the Rust code to include the classes table.
2. Pointing the Frontend to use api.ts (which talks to Rust) instead of db.ts.
