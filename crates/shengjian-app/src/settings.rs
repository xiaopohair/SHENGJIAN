use crate::ports::SettingsStorePort;
use crate::settings_namespace::SettingsDocument;
use shengjian_contract::{CoreResult, EngineError, ErrorDomain};
use shengjian_domain::AppSettings;
use std::sync::Arc;

pub struct SettingsService {
    store: Arc<dyn SettingsStorePort>,
}

impl SettingsService {
    pub fn new(store: Arc<dyn SettingsStorePort>) -> Self {
        Self { store }
    }

    pub fn load(&self) -> CoreResult<AppSettings> {
        self.store.load()
    }

    pub fn save(&self, settings: &AppSettings) -> CoreResult<()> {
        validate(settings)?;
        self.store.save(settings)
    }

    pub fn load_document(&self) -> CoreResult<SettingsDocument> {
        self.store.load_document()
    }

    pub fn save_document(&self, document: &SettingsDocument) -> CoreResult<()> {
        validate(&document.global)?;
        self.store.save_document(document)
    }
}

fn validate(settings: &AppSettings) -> CoreResult<()> {
    if settings.schema_version == 0 {
        return Err(EngineError::new(
            ErrorDomain::Configuration,
            "settings-schema-version",
        ));
    }
    Ok(())
}
