CREATE TABLE sessions_new (
 uuid TEXT PRIMARY KEY, name TEXT NOT NULL CHECK(length(trim(name)) BETWEEN 1 AND 128),
 protocol TEXT NOT NULL CHECK(protocol IN ('SSH','RDP','VNC','WEB')), hostname TEXT NOT NULL CHECK(length(hostname)>0),
 port INTEGER NOT NULL CHECK(port BETWEEN 1 AND 65535), username TEXT NOT NULL DEFAULT '', domain TEXT NOT NULL DEFAULT '',
 folder_id TEXT REFERENCES folders(uuid) ON DELETE RESTRICT, credential_id TEXT UNIQUE,
 favorite INTEGER NOT NULL DEFAULT 0 CHECK(favorite IN (0,1)), description TEXT NOT NULL DEFAULT '',
 color TEXT NOT NULL DEFAULT '#7195ff', icon TEXT NOT NULL DEFAULT 'server',
 created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
 updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')), last_connected_at TEXT
);
INSERT INTO sessions_new SELECT * FROM sessions;
DROP TABLE sessions;
ALTER TABLE sessions_new RENAME TO sessions;
CREATE INDEX sessions_folder ON sessions(folder_id);
CREATE INDEX session_identity ON sessions(protocol,hostname,port,username);
