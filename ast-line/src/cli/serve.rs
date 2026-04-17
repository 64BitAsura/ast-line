use anyhow::Result;
use clap::Args;

#[derive(Args, Debug)]
pub struct ServeCommand {
    #[arg(short, long, default_value = "4747")]
    pub port: u16,
}

pub async fn run(cmd: ServeCommand) -> Result<()> {
    crate::server::start_server(cmd.port).await
}
