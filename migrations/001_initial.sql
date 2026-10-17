CREATE TABLE folders (
 uuid TEXT PRIMARY KEY, name TEXT NOT NULL CHECK(length(trim(name)) BETWEEN 1 AND 128),
 parent_id TEXT REFERENCES folders(uuid) ON DELETE RESTRICT,
 sort_order INTEGER NOT NULL DEFAULT 0, color TEXT NOT NULL DEFAULT '#7195ff', icon TEXT NOT NULL DEFAULT 'folder',
 CHECK(parent_id IS NULL OR parent_id <> uuid)
);
CREATE TABLE sessions (
 uuid TEXT PRIMARY KEY, name TEXT NOT NULL CHECK(length(trim(name)) BETWEEN 1 AND 128),
 protocol TEXT NOT NULL CHECK(protocol IN ('SSH','RDP')), hostname TEXT NOT NULL CHECK(length(hostname)>0),
 port INTEGER NOT NULL CHECK(port BETWEEN 1 AND 65535), username TEXT NOT NULL DEFAULT '', domain TEXT NOT NULL DEFAULT '',
 folder_id TEXT REFERENCES folders(uuid) ON DELETE RESTRICT, credential_id TEXT UNIQUE,
 favorite INTEGER NOT NULL DEFAULT 0 CHECK(favorite IN (0,1)), description TEXT NOT NULL DEFAULT '',
 color TEXT NOT NULL DEFAULT '#7195ff', icon TEXT NOT NULL DEFAULT 'server',
 created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
 updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')), last_connected_at TEXT
);
CREATE INDEX sessions_folder ON sessions(folder_id);
CREATE INDEX session_identity ON sessions(protocol,hostname,port,username);
CREATE TABLE tags (name TEXT PRIMARY KEY);
CREATE TABLE session_tags (session_id TEXT NOT NULL REFERENCES sessions(uuid) ON DELETE CASCADE, tag TEXT NOT NULL REFERENCES tags(name) ON DELETE CASCADE, PRIMARY KEY(session_id,tag));
CREATE TABLE ssh_settings (
 session_id TEXT PRIMARY KEY REFERENCES sessions(uuid) ON DELETE CASCADE,
 auth_method TEXT NOT NULL DEFAULT 'agent' CHECK(auth_method IN ('agent','password','key','interactive')),
 private_key_path TEXT, passphrase_credential_id TEXT,
 keepalive INTEGER NOT NULL DEFAULT 1 CHECK(keepalive IN (0,1)), keepalive_interval INTEGER NOT NULL DEFAULT 30 CHECK(keepalive_interval > 0),
 timeout INTEGER NOT NULL DEFAULT 15 CHECK(timeout > 0)
);
CREATE TABLE rdp_settings (
 session_id TEXT PRIMARY KEY REFERENCES sessions(uuid) ON DELETE CASCADE,
 width INTEGER NOT NULL DEFAULT 1280 CHECK(width > 0), height INTEGER NOT NULL DEFAULT 800 CHECK(height > 0),
 fullscreen INTEGER NOT NULL DEFAULT 0, dynamic_resize INTEGER NOT NULL DEFAULT 1,
 multi_monitor INTEGER NOT NULL DEFAULT 0, clipboard INTEGER NOT NULL DEFAULT 0, audio INTEGER NOT NULL DEFAULT 0,
 performance TEXT NOT NULL DEFAULT 'normal' CHECK(performance IN ('lan','normal','slow')),
 certificate_policy TEXT NOT NULL DEFAULT 'verify' CHECK(certificate_policy IN ('verify','prompt'))
);
CREATE TABLE ssh_tunnels (
 uuid TEXT PRIMARY KEY, session_id TEXT NOT NULL REFERENCES sessions(uuid) ON DELETE CASCADE, name TEXT NOT NULL,
 type TEXT NOT NULL CHECK(type IN ('LOCAL','REMOTE','DYNAMIC')), bind_address TEXT NOT NULL DEFAULT '127.0.0.1',
 local_port INTEGER NOT NULL CHECK(local_port BETWEEN 1 AND 65535), remote_host TEXT,
 remote_port INTEGER CHECK(remote_port BETWEEN 1 AND 65535), auto_start INTEGER NOT NULL DEFAULT 0,
 CHECK(type = 'DYNAMIC' OR (remote_host IS NOT NULL AND remote_port IS NOT NULL))
);
CREATE TABLE known_hosts (hostname TEXT NOT NULL, port INTEGER NOT NULL CHECK(port BETWEEN 1 AND 65535), algorithm TEXT NOT NULL, public_key TEXT NOT NULL, fingerprint TEXT NOT NULL, trusted_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP, PRIMARY KEY(hostname,port,algorithm));
CREATE TABLE application_settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
INSERT INTO application_settings VALUES ('theme','system');
CREATE TABLE recent_sessions (session_id TEXT PRIMARY KEY REFERENCES sessions(uuid) ON DELETE CASCADE, connected_at TEXT NOT NULL);
CREATE TABLE transfer_history (uuid TEXT PRIMARY KEY, session_id TEXT REFERENCES sessions(uuid) ON DELETE SET NULL, direction TEXT NOT NULL, source TEXT NOT NULL, destination TEXT NOT NULL, bytes INTEGER NOT NULL DEFAULT 0, status TEXT NOT NULL, completed_at TEXT);
CREATE TABLE import_history (uuid TEXT PRIMARY KEY, format TEXT NOT NULL, imported INTEGER NOT NULL, skipped INTEGER NOT NULL, errors INTEGER NOT NULL, created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP);
