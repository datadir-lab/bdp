use clap::Parser;

#[derive(Debug, Clone, PartialEq)]
pub enum Transport {
    Stdio,
    Http,
}

impl std::str::FromStr for Transport {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "stdio" => Ok(Transport::Stdio),
            "http" => Ok(Transport::Http),
            other => Err(format!("Unknown transport: {other}. Use 'stdio' or 'http'")),
        }
    }
}

#[derive(Debug, Clone, Parser)]
#[command(name = "bdp-mcp", about = "BDP MCP server")]
pub struct Config {
    /// Transport mode: stdio (default) or http
    #[arg(long, env = "BDP_MCP_TRANSPORT", default_value = "stdio")]
    pub transport: Transport,

    /// HTTP port (used when transport=http)
    #[arg(long, env = "BDP_MCP_PORT", default_value = "3000")]
    pub port: u16,

    /// PostgreSQL connection URL
    #[arg(long, env = "DATABASE_URL")]
    pub database_url: String,

    /// Max DB connections
    #[arg(long, env = "DB_MAX_CONNECTIONS", default_value = "10")]
    pub db_max_connections: u32,
}

impl Config {
    pub fn from_env_and_args(args: &[String]) -> Self {
        Config::parse_from(args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        std::env::set_var("DATABASE_URL", "postgresql://test");
        let cfg = Config::from_env_and_args(&["bdp-mcp".to_string()]);
        assert_eq!(cfg.transport, Transport::Stdio);
        assert_eq!(cfg.port, 3000);
    }
}
