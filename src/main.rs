use anyhow::Result;
use clap::{Parser, Subcommand};
use genai::llm::client::LlmClient;
use genai::llm::config::LlmConfig;
use genai::llm::gemini::GeminiLlmClient;
use genai::llm::mock::MockLlmClient;
use genai::skill::scanner::scan_skills;
use genai::skill::selector::select_skill;
use genai::skill::validator::validate_skill;
use genai::workflow::executor::{ExecutionInput, WorkflowExecutor};
use tracing::{debug, info, warn};

#[derive(Parser, Debug)]
#[command(name = "genai")]
#[command(about = "Skill-based Rust agent runtime")]
struct Cli {
    #[arg(long, default_value = "./skills")]
    skills_dir: String,

    #[arg(long, default_value_t = false)]
    debug: bool,

    #[arg(long, default_value_t = false)]
    real_llm: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    List,
    Run { prompt: String },
    RunSkill { skill_name: String, prompt: String },
}

fn init_tracing(debug_mode: bool) {
    let filter = if debug_mode { "debug" } else { "info" };
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();
}

fn build_llm_client(real_llm: bool) -> Box<dyn LlmClient> {
    dotenvy::dotenv().ok();

    let has_key = std::env::var("GEMINI_API_KEY").is_ok();
    let should_use_real = real_llm || has_key;

    if should_use_real {
        match LlmConfig::from_env().and_then(GeminiLlmClient::new) {
            Ok(client) => {
                info!("Using GeminiLlmClient");
                return Box::new(client);
            }
            Err(err) => {
                warn!("Unable to initialize GeminiLlmClient, falling back to mock: {err}");
            }
        }
    }

    info!("Using MockLlmClient");
    Box::new(MockLlmClient::new())
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    init_tracing(cli.debug);

    let skills = scan_skills(&cli.skills_dir)?;
    for skill in &skills {
        validate_skill(skill)?;
    }

    match cli.command {
        Commands::List => {
            for skill in &skills {
                println!(
                    "{} ({}) - {}",
                    skill.metadata.name, skill.metadata.version, skill.metadata.description
                );
            }
        }
        Commands::Run { prompt } => {
            let selector_llm = build_llm_client(cli.real_llm);
            let selected = select_skill(&prompt, &skills, Some(selector_llm.as_ref()))?;
            info!("Selected skill: {}", selected.metadata.name);

            // let mut executor = WorkflowExecutor::new(build_llm_client(cli.real_llm));
            // let result = executor.execute(
            //     selected,
            //     ExecutionInput {
            //         user_prompt: prompt,
            //         debug: cli.debug,
            //     },
            // )?;
            // println!("{result}");
        }
        Commands::RunSkill { skill_name, prompt } => {
            let skill = skills
                .iter()
                .find(|s| s.metadata.name == skill_name)
                .ok_or_else(|| anyhow::anyhow!("Skill not found: {skill_name}"))?;

            debug!("Running skill: {}", skill.metadata.name);
            let mut executor = WorkflowExecutor::new(build_llm_client(cli.real_llm));
            let result = executor.execute(
                skill,
                ExecutionInput {
                    user_prompt: prompt,
                    debug: cli.debug,
                },
            )?;
            println!("{result}");
        }
    }

    Ok(())
}
