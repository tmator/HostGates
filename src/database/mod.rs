use crate::{
    error::{AppError, Result},
    models::*,
};
use rusqlite::{Connection, OptionalExtension, params};
use std::path::Path;
mod import;

pub struct Database {
    conn: Connection,
}
impl Database {
    pub fn open(path: &Path) -> Result<Self> {
        Self::from_connection(Connection::open(path)?)
    }
    pub fn in_memory() -> Result<Self> {
        Self::from_connection(Connection::open_in_memory()?)
    }
    fn from_connection(mut conn: Connection) -> Result<Self> {
        conn.busy_timeout(std::time::Duration::from_secs(3))?;
        conn.execute_batch(
            "PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;",
        )?;
        let version: u32 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        if version > 2 {
            return Err(AppError::Validation(
                "Base créée par une version plus récente de HostGates.".into(),
            ));
        }
        if version == 0 {
            let tx = conn.transaction()?;
            tx.execute_batch(include_str!("../../migrations/001_initial.sql"))?;
            tx.pragma_update(None, "user_version", 1)?;
            tx.commit()?;
        }
        if version < 2 {
            conn.pragma_update(None, "foreign_keys", false)?;
            let migration = (|| -> Result<()> {
                let tx = conn.transaction()?;
                tx.execute_batch(include_str!("../../migrations/002_vnc_web.sql"))?;
                let violations: i64 =
                    tx.query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |r| {
                        r.get(0)
                    })?;
                if violations != 0 {
                    return Err(AppError::Database);
                }
                tx.pragma_update(None, "user_version", 2)?;
                tx.commit()?;
                Ok(())
            })();
            conn.pragma_update(None, "foreign_keys", true)?;
            migration?;
        }
        Ok(Self { conn })
    }
    pub fn snapshot(&self) -> Result<Snapshot> {
        let mut stmt = self.conn.prepare("SELECT uuid,name,protocol,hostname,port,username,domain,folder_id,favorite,description,color,icon,credential_id,created_at,updated_at,last_connected_at FROM sessions ORDER BY name COLLATE NOCASE")?;
        let mut sessions = stmt
            .query_map([], |r| {
                Ok(Session {
                    uuid: r.get(0)?,
                    data: SessionInput {
                        name: r.get(1)?,
                        protocol: match r.get::<_, String>(2)?.as_str() {
                            "SSH" => Protocol::SSH,
                            "RDP" => Protocol::RDP,
                            "VNC" => Protocol::VNC,
                            "WEB" => Protocol::WEB,
                            _ => return Err(rusqlite::Error::InvalidQuery),
                        },
                        hostname: r.get(3)?,
                        port: r.get(4)?,
                        username: r.get(5)?,
                        domain: r.get(6)?,
                        folder_id: r.get(7)?,
                        favorite: r.get(8)?,
                        description: r.get(9)?,
                        color: r.get(10)?,
                        icon: r.get(11)?,
                        tags: vec![],
                    },
                    credential_id: r.get(12)?,
                    created_at: r.get(13)?,
                    updated_at: r.get(14)?,
                    last_connected_at: r.get(15)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let mut tags = self
            .conn
            .prepare("SELECT tag FROM session_tags WHERE session_id=? ORDER BY tag")?;
        for s in &mut sessions {
            s.data.tags = tags
                .query_map([&s.uuid], |r| r.get(0))?
                .collect::<std::result::Result<Vec<_>, _>>()?;
        }
        let mut stmt = self.conn.prepare("SELECT uuid,name,parent_id,sort_order,color,icon FROM folders ORDER BY sort_order,name COLLATE NOCASE")?;
        let folders = stmt
            .query_map([], |r| {
                Ok(Folder {
                    uuid: r.get(0)?,
                    data: FolderInput {
                        name: r.get(1)?,
                        parent_id: r.get(2)?,
                        sort_order: r.get(3)?,
                        color: r.get(4)?,
                        icon: r.get(5)?,
                    },
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(Snapshot {
            programs: self.programs()?,
            sessions,
            folders,
            theme: self.conn.query_row(
                "SELECT value FROM application_settings WHERE key='theme'",
                [],
                |r| r.get(0),
            )?,
        })
    }
    pub fn session(&self, id: &str) -> Result<Session> {
        self.snapshot()?
            .sessions
            .into_iter()
            .find(|s| s.uuid == id)
            .ok_or(AppError::NotFound)
    }
    pub fn save_session(&mut self, id: Option<&str>, data: &SessionInput) -> Result<String> {
        data.validate()?;
        let uuid = match id {
            Some(id) => {
                let exists: bool = self.conn.query_row(
                    "SELECT EXISTS(SELECT 1 FROM sessions WHERE uuid=?)",
                    [id],
                    |r| r.get(0),
                )?;
                if !exists {
                    return Err(AppError::NotFound);
                }
                id.to_owned()
            }
            None => uuid::Uuid::new_v4().to_string(),
        };
        let tx = self.conn.savepoint()?;
        tx.execute("INSERT INTO sessions(uuid,name,protocol,hostname,port,username,domain,folder_id,favorite,description,color,icon) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12) ON CONFLICT(uuid) DO UPDATE SET name=excluded.name,protocol=excluded.protocol,hostname=excluded.hostname,port=excluded.port,username=excluded.username,domain=excluded.domain,folder_id=excluded.folder_id,favorite=excluded.favorite,description=excluded.description,color=excluded.color,icon=excluded.icon,updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now')",
            params![uuid,data.name.trim(),data.protocol.as_str(),data.hostname,data.port,data.username,data.domain,data.folder_id,data.favorite,data.description,data.color,data.icon])?;
        // No stale protocol settings when switching a session type.
        for (protocol, table) in [
            (Protocol::SSH, "ssh_settings"),
            (Protocol::RDP, "rdp_settings"),
        ] {
            if data.protocol == protocol {
                tx.execute(
                    &format!("INSERT OR IGNORE INTO {table}(session_id) VALUES (?)"),
                    [&uuid],
                )?;
            } else {
                tx.execute(&format!("DELETE FROM {table} WHERE session_id=?"), [&uuid])?;
            }
        }
        if data.protocol != Protocol::SSH {
            tx.execute("DELETE FROM ssh_tunnels WHERE session_id=?", [&uuid])?;
        }
        tx.execute("DELETE FROM session_tags WHERE session_id=?", [&uuid])?;
        for tag in &data.tags {
            tx.execute("INSERT OR IGNORE INTO tags(name) VALUES (?)", [tag.trim()])?;
            tx.execute(
                "INSERT OR IGNORE INTO session_tags VALUES (?,?)",
                params![uuid, tag.trim()],
            )?;
        }
        tx.commit()?;
        Ok(uuid)
    }
    pub fn link_credential(&mut self, id: &str, credential: Option<&str>) -> Result<()> {
        if self.conn.execute(
            "UPDATE sessions SET credential_id=? WHERE uuid=?",
            params![credential, id],
        )? != 1
        {
            return Err(AppError::NotFound);
        }
        Ok(())
    }
    pub fn move_session(&mut self, id: &str, folder: Option<&str>) -> Result<()> {
        if let Some(folder) = folder {
            validate_id(folder)?;
            let exists: bool = self.conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM folders WHERE uuid=?)",
                [folder],
                |r| r.get(0),
            )?;
            if !exists {
                return Err(AppError::NotFound);
            }
        }
        if self.conn.execute("UPDATE sessions SET folder_id=?1,updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE uuid=?2", params![folder, id])? != 1 { return Err(AppError::NotFound); }
        Ok(())
    }
    pub fn delete_session(&mut self, id: &str) -> Result<()> {
        if self
            .conn
            .execute("DELETE FROM sessions WHERE uuid=?", [id])?
            != 1
        {
            return Err(AppError::NotFound);
        }
        Ok(())
    }
    pub fn save_folder(&mut self, id: Option<&str>, data: &FolderInput) -> Result<String> {
        validate_name(&data.name)?;
        validate_color(&data.color)?;
        let folders = self.snapshot()?.folders;
        let uuid = match id {
            Some(id) => {
                if !folders.iter().any(|f| f.uuid == id) {
                    return Err(AppError::NotFound);
                }
                id.to_owned()
            }
            None => uuid::Uuid::new_v4().to_string(),
        };
        let mut parent = data.parent_id.as_deref();
        let mut depth = 0;
        while let Some(p) = parent {
            if p == uuid || depth >= 64 {
                return Err(AppError::Validation(
                    "Cycle ou profondeur de dossiers excessive (maximum 64).".into(),
                ));
            }
            parent = folders
                .iter()
                .find(|f| f.uuid == p)
                .ok_or(AppError::NotFound)?
                .data
                .parent_id
                .as_deref();
            depth += 1;
        }
        // Also account for the depth of an existing subtree moved below this parent.
        for folder in &folders {
            let mut cursor = Some(folder.uuid.as_str());
            let mut below = 0;
            while let Some(p) = cursor {
                if p == uuid {
                    if depth + below >= 64 {
                        return Err(AppError::Validation(
                            "Profondeur maximale de 64 dossiers.".into(),
                        ));
                    }
                    break;
                }
                if below >= 64 {
                    return Err(AppError::Validation("Arborescence invalide.".into()));
                }
                cursor = folders
                    .iter()
                    .find(|f| f.uuid == p)
                    .and_then(|f| f.data.parent_id.as_deref());
                below += 1;
            }
        }
        self.conn.execute("INSERT INTO folders(uuid,name,parent_id,sort_order,color,icon) VALUES (?1,?2,?3,?4,?5,?6) ON CONFLICT(uuid) DO UPDATE SET name=excluded.name,parent_id=excluded.parent_id,sort_order=excluded.sort_order,color=excluded.color,icon=excluded.icon", params![uuid,data.name.trim(),data.parent_id,data.sort_order,data.color,data.icon])?;
        Ok(uuid)
    }
    pub fn delete_folder(&mut self, id: &str) -> Result<()> {
        let count: i64 = self.conn.query_row("SELECT (SELECT count(*) FROM folders WHERE parent_id=?1)+(SELECT count(*) FROM sessions WHERE folder_id=?1)", [id], |r| r.get(0))?;
        if count > 0 {
            return Err(AppError::Validation(
                "Ce dossier contient des éléments. Déplacez-les avant de le supprimer.".into(),
            ));
        }
        if self
            .conn
            .execute("DELETE FROM folders WHERE uuid=?", [id])?
            != 1
        {
            return Err(AppError::NotFound);
        }
        Ok(())
    }
    pub fn programs(&self) -> Result<ExternalPrograms> {
        let read = |key: &str| -> Result<String> {
            Ok(self
                .conn
                .query_row(
                    "SELECT value FROM application_settings WHERE key=?",
                    [key],
                    |r| r.get(0),
                )
                .optional()?
                .unwrap_or_default())
        };
        Ok(ExternalPrograms {
            ssh: read("program_ssh")?,
            rdp: read("program_rdp")?,
            vnc: read("program_vnc")?,
            terminal: read("program_terminal")?,
        })
    }
    pub fn set_programs(&mut self, programs: &ExternalPrograms) -> Result<()> {
        programs.validate()?;
        let tx = self.conn.savepoint()?;
        for (key, value) in [
            ("program_ssh", &programs.ssh),
            ("program_rdp", &programs.rdp),
            ("program_vnc", &programs.vnc),
            ("program_terminal", &programs.terminal),
        ] {
            tx.execute("INSERT INTO application_settings(key,value) VALUES (?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value", params![key, value])?;
        }
        tx.commit()?;
        Ok(())
    }
    pub fn set_theme(&mut self, theme: &str) -> Result<()> {
        if !["dark", "light", "system"].contains(&theme) {
            return Err(AppError::Validation("Thème inconnu.".into()));
        }
        self.conn.execute(
            "UPDATE application_settings SET value=? WHERE key='theme'",
            [theme],
        )?;
        Ok(())
    }
    pub fn schema_version(&self) -> Result<u32> {
        Ok(self
            .conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))?)
    }
    pub fn credential_reference(&self, id: &str) -> Result<Option<String>> {
        self.conn
            .query_row(
                "SELECT credential_id FROM sessions WHERE uuid=?",
                [id],
                |r| r.get(0),
            )
            .optional()?
            .ok_or(AppError::NotFound)
    }
}
