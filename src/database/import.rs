use super::*;
use crate::importers::{DuplicatePolicy, Preview, exchange::identity, folder_parts};
#[derive(Default)]
pub struct ImportReport {
    pub ssh: usize,
    pub rdp: usize,
    pub vnc: usize,
    pub web: usize,
    pub skipped: usize,
    pub unsupported: usize,
    pub invalid: usize,
    pub replaced: usize,
}
impl ImportReport {
    pub fn summary(&self) -> String {
        format!(
            "Import terminé : {} SSH, {} RDP, {} VNC, {} Web ; {} remplacées ; {} ignorées ; {} non prises en charge ; {} invalides.",
            self.ssh,
            self.rdp,
            self.vnc,
            self.web,
            self.replaced,
            self.skipped,
            self.unsupported,
            self.invalid
        )
    }
}
impl Database {
    pub fn import(&mut self, preview: &Preview) -> Result<ImportReport> {
        self.conn.execute_batch("SAVEPOINT import_batch")?;
        let result = self.import_inner(preview);
        match result {
            Ok(report) => {
                self.conn.execute_batch("RELEASE import_batch")?;
                Ok(report)
            }
            Err(e) => {
                self.conn
                    .execute_batch("ROLLBACK TO import_batch; RELEASE import_batch")?;
                Err(e)
            }
        }
    }
    fn import_inner(&mut self, preview: &Preview) -> Result<ImportReport> {
        let mut report = ImportReport::default();
        let snapshot = self.snapshot()?;
        let mut sessions = std::collections::HashMap::<_, Vec<String>>::new();
        for session in snapshot.sessions {
            sessions
                .entry(identity(&session.data))
                .or_default()
                .push(session.uuid);
        }
        let mut folders: std::collections::HashMap<_, _> = snapshot
            .folders
            .into_iter()
            .map(|f| ((f.data.parent_id, f.data.name), f.uuid))
            .collect();
        for row in &preview.rows {
            let Some(mut data) = row.data.clone() else {
                if ["SSH", "RDP", "VNC", "WEB", "?"].contains(&row.kind.as_str()) {
                    report.invalid += 1
                } else {
                    report.unsupported += 1
                }
                continue;
            };
            if !row.selected {
                report.skipped += 1;
                continue;
            }
            let key = identity(&data);
            let duplicates = sessions.get(&key);
            let duplicate_count = duplicates.map_or(0, Vec::len);
            let first_duplicate = duplicates.and_then(|ids| ids.first()).cloned();
            if duplicate_count != 0 && row.policy == DuplicatePolicy::Skip {
                report.skipped += 1;
                continue;
            }
            if duplicate_count > 1 && row.policy == DuplicatePolicy::Replace {
                return Err(AppError::Validation(format!(
                    "Plusieurs sessions correspondent à « {} ». Choisissez Ignorer ou Importer quand même.",
                    data.name
                )));
            }
            let mut parent = None;
            for part in folder_parts(&row.folder)? {
                let folder_key = (parent.clone(), part.to_owned());
                let id = if let Some(id) = folders.get(&folder_key) {
                    id.clone()
                } else {
                    // folder_parts already validates names/depth. These are fresh children,
                    // never subtree moves, so no full-tree cycle scan is needed.
                    let id = uuid::Uuid::new_v4().to_string();
                    self.conn.execute(
                        "INSERT INTO folders(uuid,name,parent_id,sort_order,color,icon) VALUES (?,?,?,0,'#7195ff','folder')",
                        params![id, part.trim(), parent],
                    )?;
                    folders.insert(folder_key, id.clone());
                    id
                };
                parent = Some(id);
            }
            data.folder_id = parent;
            let replace = if row.policy == DuplicatePolicy::Replace {
                first_duplicate.as_deref()
            } else {
                None
            };
            let id = self.save_session(replace, &data)?;
            if replace.is_none() {
                sessions.entry(key).or_default().push(id);
            }
            if replace.is_some() {
                report.replaced += 1;
            }
            match data.protocol {
                Protocol::SSH => report.ssh += 1,
                Protocol::RDP => report.rdp += 1,
                Protocol::VNC => report.vnc += 1,
                Protocol::WEB => report.web += 1,
            }
        }
        self.conn.execute(
            "INSERT INTO import_history(uuid,format,imported,skipped,errors) VALUES (?,?,?,?,?)",
            params![
                uuid::Uuid::new_v4().to_string(),
                "mobaxterm",
                (report.ssh + report.rdp + report.vnc + report.web) as i64,
                (report.skipped + report.unsupported) as i64,
                report.invalid as i64
            ],
        )?;
        Ok(report)
    }
}
