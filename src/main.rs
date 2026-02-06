use anyhow::Result;
use clap::{Parser, Subcommand};
use genai::llm::mock::MockLlmClient;
use genai::skill::scanner::scan_skills;
use genai::skill::selector::select_skill;
use genai::skill::validator::validate_skill;
use genai::workflow::executor::{ExecutionInput, WorkflowExecutor};
use tracing::{debug, info};

#[derive(Parser, Debug)]
#[command(name = "genai")]
#[command(about = "Skill-based Rust agent runtime")]
struct Cli {
    #[arg(long, default_value = "./skills")]
    skills_dir: String,

    #[arg(long, default_value_t = false)]
    debug: bool,

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
            let llm = MockLlmClient::new();
            let selected = select_skill(&prompt, &skills, Some(&llm))?;
            info!("Selected skill: {}", selected.metadata.name);

            let mut executor = WorkflowExecutor::new(Box::new(llm));
            let result = executor.execute(
                selected,
                ExecutionInput {
                    user_prompt: prompt,
                    debug: cli.debug,
                },
            )?;
            println!("{result}");
        }
        Commands::RunSkill { skill_name, prompt } => {
            let skill = skills
                .iter()
                .find(|s| s.metadata.name == skill_name)
                .ok_or_else(|| anyhow::anyhow!("Skill not found: {skill_name}"))?;

            debug!("Running skill: {}", skill.metadata.name);
            let mut executor = WorkflowExecutor::new(Box::new(MockLlmClient::new()));
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
