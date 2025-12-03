pub mod config;
pub mod telemetry;
pub mod plugin_manager;

use anyhow::Result;
use config::Config;
use parking_lot::RwLock;
use plugin_manager::PluginManager;
use std::collections::HashMap;
use std::sync::Arc;
use sunrush_bus::MessageBus;
use telemetry::TelemetryServer;
use tracing::{info, error};

pub struct Host {
    config: Config,
    bus: Arc<MessageBus>,
    telemetry: Arc<TelemetryServer>,
    plugin_manager: Arc<PluginManager>,
}

impl Host {
    pub fn new(config: Config) -> Result<Self> {
        // Initialize message bus
        let bus = Arc::new(MessageBus::new(config.host.bus_capacity));
        
        // Initialize telemetry
        let telemetry = Arc::new(TelemetryServer::new()?);
        
        // Prepare plugin configs
        let plugin_configs = Arc::new(RwLock::new(config.plugins.clone()));
        
        // Initialize plugin manager
        let plugin_manager = PluginManager::new(
            Arc::clone(&bus),
            Arc::clone(&telemetry),
            config.host.plugin_dir.clone(),
            plugin_configs,
        );

        Ok(Self {
            config,
            bus,
            telemetry,
            plugin_manager,
        })
    }

    pub async fn run(self) -> Result<()> {
        info!("Starting SunRush Host");
        
        // Start telemetry server if enabled
        let telemetry_handle = if self.config.telemetry.enabled {
            let telemetry = Arc::clone(&self.telemetry);
            let bind = self.config.telemetry.bind.clone();
            let port = self.config.telemetry.port;
            
            Some(tokio::spawn(async move {
                if let Err(e) = telemetry.serve(bind, port).await {
                    error!("Telemetry server error: {}", e);
                }
            }))
        } else {
            None
        };

        // Load all plugins
        info!("Loading plugins from {:?}", self.config.host.plugin_dir);
        self.plugin_manager.load_all_plugins()?;
        
        // Start all plugins
        info!("Starting plugins");
        self.plugin_manager.start_all_plugins()?;
        
        info!("SunRush Host is running");
        
        // Wait for shutdown signal
        tokio::signal::ctrl_c().await?;
        
        info!("Shutting down SunRush Host");
        
        // Stop all plugins
        self.plugin_manager.stop_all_plugins()?;
        
        // Wait for telemetry server to shutdown
        if let Some(handle) = telemetry_handle {
            handle.abort();
        }
        
        info!("SunRush Host stopped");
        
        Ok(())
    }
}

