use crate::models::AppConfig;
use config::{Config, ConfigError, Environment, File};

pub fn load_config() -> Result<AppConfig, ConfigError> {
    // Carrega o arquivo .env silenciosamente se existir
    dotenvy::dotenv().ok();

    // Define o ambiente alvo (padrão é development)
    let run_env = std::env::var("RUN_ENV").unwrap_or_else(|_| "development".into());

    let builder = Config::builder()
        // Carrega as configurações base
        .add_source(File::with_name("config/default.toml"))
        // Sobrescreve com as configurações específicas do ambiente
        .add_source(File::with_name(&format!("config/{}.toml", run_env)).required(false))
        // Variáveis de ambiente têm prioridade máxima
        .add_source(Environment::default().separator("__"));

    builder.build()?.try_deserialize()
}