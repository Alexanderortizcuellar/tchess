use scid_mgr::db::{GameFilter, GameSummary};
use scid_mgr::pgn_db::PgnDatabaseWrapper;
use scid_mgr::search::evaluator::GameSearchEvaluator;
use scid_mgr::search::parser::QueryParser;
use std::path::{Path, PathBuf};

pub enum DbBackend {
    Pgn(PgnDatabaseWrapper),
}

pub struct DatabaseManager {
    pub path: PathBuf,
    pub backend: DbBackend,
}

impl DatabaseManager {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let p = path.as_ref().to_path_buf();
        let pgn_db = PgnDatabaseWrapper::open(&p).map_err(|e| format!("{}", e))?;
        Ok(Self {
            path: p,
            backend: DbBackend::Pgn(pgn_db),
        })
    }

    pub fn game_count(&self) -> usize {
        match &self.backend {
            DbBackend::Pgn(db) => db.game_count(),
        }
    }

    pub fn get_summary(&self, game_id: usize) -> Option<GameSummary> {
        match &self.backend {
            DbBackend::Pgn(db) => {
                if game_id < db.game_count() {
                    Some(db.get_summary(game_id))
                } else {
                    None
                }
            }
        }
    }

    pub fn get_game_pgn(&self, game_id: usize) -> Result<String, String> {
        match &self.backend {
            DbBackend::Pgn(db) => db.get_game_pgn(game_id).map_err(|e| format!("{}", e)),
        }
    }

    #[allow(dead_code)]
    pub fn query_games(
        &self,
        filter: &GameFilter,
        page: usize,
        page_size: usize,
    ) -> (Vec<GameSummary>, usize) {
        match &self.backend {
            DbBackend::Pgn(db) => db.query_games(filter, page, page_size),
        }
    }

    pub fn execute_cql_query(
        &self,
        query_str: &str,
        max_results: usize,
    ) -> Result<Vec<(usize, GameSummary)>, String> {
        let query = QueryParser::parse_str(query_str).map_err(|e| format!("{}", e))?;
        let mut matches = Vec::new();

        match &self.backend {
            DbBackend::Pgn(db) => {
                for (id, entry) in db.entries.iter().enumerate() {
                    // Fast header pre-filter using scid-mgr CompactPgnRecord
                    if let Some(false) = scid_mgr::search::evaluator::quick_check_pgn_entry_headers(
                        &query, entry, &db.names,
                    ) {
                        continue;
                    }
                    if let Ok(pgn_text) = db.get_game_pgn(id) {
                        let res = GameSearchEvaluator::evaluate_pgn(&query, &pgn_text);
                        if res.is_match {
                            matches.push((id, db.get_summary(id)));
                            if matches.len() >= max_results {
                                break;
                            }
                        }
                    }
                }
            }
        }

        Ok(matches)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;

    #[test]
    fn test_database_manager_pgn() {
        let pgn_content = r#"[Event "World Championship"]
[Site "Reykjavik ISL"]
[Date "1972.07.23"]
[White "Fischer, Robert J."]
[Black "Spassky, Boris V."]
[Result "1-0"]
[ECO "E41"]

1. d4 Nf6 2. c4 e6 3. Nc3 Bb4 4. e3 c5 5. Bd3 Nc6 1-0

[Event "Opera House"]
[Site "Paris FRA"]
[Date "1858.??.??"]
[White "Morphy, Paul"]
[Black "Duke Karl / Count Isouard"]
[Result "1-0"]
[ECO "C41"]

1. e4 e5 2. Nf3 d6 3. d4 Bg4 4. dxe5 Bxf3 1-0
"#;
        let temp_path = std::env::temp_dir().join("tchess_test_db.pgn");
        let mut f = fs::File::create(&temp_path).unwrap();
        f.write_all(pgn_content.as_bytes()).unwrap();
        drop(f);

        let db = DatabaseManager::open(&temp_path).unwrap();
        assert_eq!(db.game_count(), 2);

        let g0 = db.get_summary(0).unwrap();
        assert!(g0.white.contains("Fischer"));

        let g1 = db.get_summary(1).unwrap();
        assert!(g1.white.contains("Morphy"));

        // Test CQL query execution with exact match and result match
        let matches = db.execute_cql_query("white=\"Morphy, Paul\"", 10).unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].0, 1);

        let res_matches = db.execute_cql_query("result=\"1-0\"", 10).unwrap();
        assert_eq!(res_matches.len(), 2);

        let _ = fs::remove_file(&temp_path);
        let _ = fs::remove_file(DatabaseManager::open(&temp_path).err().unwrap_or_default());
    }
}
