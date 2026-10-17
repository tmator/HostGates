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
            let snapshot = self.snapshot()?;
            let duplicates: Vec<_> = snapshot
                .sessions
                .iter()
                .filter(|s| identity(&s.data) == identity(&data))
                .collect();
            if !duplicates.is_empty() && row.policy == DuplicatePolicy::Skip {
                report.skipped += 1;
                continue;
            }
            if duplicates.len() > 1 && row.policy == DuplicatePolicy::Replace {
                return Err(AppError::Validation(format!(
                    "Plusieurs sessions correspondent à « {} ». Choisissez Ignorer ou Importer quand même.",
                    data.name
                )));
            }
            let mut parent = None;
            for part in folder_parts(&row.folder)? {
                let folders = self.snapshot()?.folders;
                parent = Some(
                    match folders
                        .iter()
                        .find(|f| f.data.name == part && f.data.parent_id == parent)
                    {
                        Some(f) => f.uuid.clone(),
                        None => self.save_folder(
                            None,
                            &FolderInput {
                                name: part.into(),
                                parent_id: parent,
                                sort_order: 0,
                                color: "#7195ff".into(),
                                icon: "folder".into(),
                            },
                        )?,
                    },
                );
            }
            data.folder_id = parent;
            let replace = if row.policy == DuplicatePolicy::Replace {
                duplicates.first().map(|s| s.uuid.as_str())
            } else {
                None
            };
            self.save_session(replace, &data)?;
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
