//! TUI application state

use agentroot_core::{Database, SearchOptions, SearchResult};
use std::rc::Rc;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AppMode {
    Search,
    Results,
    Preview,
    Collections,
    Help,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SearchMode {
    Bm25,
    Vector,
    Hybrid,
}

pub struct App {
    pub db: Rc<Database>,
    pub mode: AppMode,
    pub search_mode: SearchMode,

    pub query: String,
    pub cursor_pos: usize,

    pub results: Vec<SearchResult>,
    pub selected: usize,
    pub scroll_offset: usize,

    pub preview_content: Option<String>,
    pub preview_scroll: usize,

    pub collection_filter: Option<String>,
    pub provider_filter: Option<String>,
    pub collections: Vec<String>,
    pub collections_selected: usize,

    pub status_message: Option<String>,
    pub is_loading: bool,

    pub should_quit: bool,
}

impl App {
    pub fn new(db: Database) -> Self {
        Self {
            db: Rc::new(db),
            mode: AppMode::Search,
            search_mode: SearchMode::Bm25,
            query: String::new(),
            cursor_pos: 0,
            results: Vec::new(),
            selected: 0,
            scroll_offset: 0,
            preview_content: None,
            preview_scroll: 0,
            collection_filter: None,
            provider_filter: None,
            collections: Vec::new(),
            collections_selected: 0,
            status_message: None,
            is_loading: false,
            should_quit: false,
        }
    }

    pub fn search(&mut self) {
        if self.query.is_empty() {
            self.results.clear();
            return;
        }

        self.is_loading = true;
        let options = SearchOptions {
            limit: 50,
            min_score: 0.0,
            collection: self.collection_filter.clone(),
            provider: self.provider_filter.clone(),
            metadata_filters: Vec::new(),
            detail: agentroot_core::DetailLevel::L2,
            ..Default::default()
        };

        match self.db.search_fts(&self.query, &options) {
            Ok(r) => {
                self.results = r;
                self.selected = 0;
                self.scroll_offset = 0;
            }
            Err(e) => {
                self.status_message = Some(format!("Search error: {}", e));
            }
        }

        self.is_loading = false;
    }

    pub fn load_collections(&mut self) {
        match self.db.list_collections() {
            Ok(colls) => {
                self.collections = colls.iter().map(|c| c.name.clone()).collect();
                self.collections_selected = 0;
            }
            Err(e) => {
                self.status_message = Some(format!("Error loading collections: {}", e));
            }
        }
    }

    pub fn toggle_collection_filter(&mut self) {
        if self.collections.is_empty() {
            self.load_collections();
        }

        if let Some(coll) = self.collections.get(self.collections_selected) {
            if self.collection_filter.as_ref() == Some(coll) {
                self.collection_filter = None;
                self.status_message = Some("Collection filter cleared".to_string());
            } else {
                self.collection_filter = Some(coll.clone());
                self.status_message = Some(format!("Filtering by collection: {}", coll));
            }
            self.search();
        }
    }

    pub fn select_next(&mut self) {
        if self.selected < self.results.len().saturating_sub(1) {
            self.selected += 1;
            self.ensure_visible();
        }
    }

    pub fn select_prev(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            self.ensure_visible();
        }
    }

    fn ensure_visible(&mut self) {
        let visible_height = 10;
        if self.selected < self.scroll_offset {
            self.scroll_offset = self.selected;
        } else if self.selected >= self.scroll_offset + visible_height {
            self.scroll_offset = self.selected - visible_height + 1;
        }
    }

    pub fn load_preview(&mut self) {
        if let Some(result) = self.results.get(self.selected) {
            self.preview_content = result.body.clone();
            self.preview_scroll = 0;
        }
    }

    pub fn cycle_search_mode(&mut self) {
        self.search_mode = match self.search_mode {
            SearchMode::Bm25 => SearchMode::Vector,
            SearchMode::Vector => SearchMode::Hybrid,
            SearchMode::Hybrid => SearchMode::Bm25,
        };
    }
}
