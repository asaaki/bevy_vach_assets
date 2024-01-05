//! BVA CLI
use std::path::PathBuf;

use bevy_vach_assets::{ARCHIVE_DIR, ASSETS_DIR, SECRETS_DIR};
use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "bva",
    version,
    about,
//     help_template = "\
// {before-help}{name} {version}
// {author-with-newline}{about-with-newline}
// {usage-heading} {usage}

// {all-args}{after-help}
// "
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[command(flatten)]
    global: GlobalArgs,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Create a new archive
    Archive {
        /// Check archive file list after creation
        #[arg(short, long)]
        check: bool,
    },

    /// Check archive file list
    #[command(name = "check")]
    CheckFiles {},

    /// Generate keys for encryption and signing
    #[command(name = "generate")]
    GenerateKeys {},
}

#[derive(Args, Clone, Debug)]
struct GlobalArgs {
    #[arg(global = true, env = "BVA_ASSETS_DIR", short, long, default_value = ASSETS_DIR)]
    assets_dir: PathBuf,

    #[arg(global = true, env = "BVA_ARCHIVE_DIR", short = 'r', long, default_value = ARCHIVE_DIR)]
    assets_archive_dir: PathBuf,

    #[arg(global = true, env = "BVA_SECRETS_DIR", short, long, default_value = SECRETS_DIR)]
    secrets_dir: PathBuf,
}

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();
    let globals = &args.global;

    match args.command {
        Commands::Archive { check, .. } => {
            archive::run(globals)?;
            if check {
                check_files::run(globals)?;
            }
        }

        Commands::CheckFiles { .. } => {
            check_files::run(globals)?;
        }

        Commands::GenerateKeys { .. } => {
            generate::run(globals)?;
        }
    }

    Ok(())
}

mod archive {
    use crate::GlobalArgs;
    use bevy_vach_assets::{
        vach::{
            builder::{CompressMode, CompressionAlgorithm, Leaf},
            prelude::{Builder, BuilderConfig, SigningKey},
            SIGNATURE_LENGTH,
        },
        ARCHIVE_MAGIC, ASSETS_ARCHIVE, ASSETS_DIR, ASSET_FILE_INDEX, ASSET_FILE_INDEX_SEP,
        SECRETS_KEY_PAIR,
    };
    use normpath::PathExt;
    use path_slash::PathExt as _;
    use std::{
        env::current_dir,
        fs::File,
        io::{Cursor, Read},
    };
    use walkdir::{DirEntry, WalkDir};

    pub(crate) fn run(globals: &GlobalArgs) -> anyhow::Result<()> {
        let dir = current_dir()?;
        let assets_path = dir.join(&globals.assets_dir);
        let archive_path = dir
            .join(&globals.assets_archive_dir)
            .join(ASSETS_ARCHIVE)
            .normalize_virtually()?
            .into_path_buf();
        let key_pair_path = dir.join(&globals.secrets_dir).join(SECRETS_KEY_PAIR);
        let mut issues = Vec::new();

        if !assets_path.exists() {
            issues.push(format!(
                "Asset directory '{}' does not exist",
                assets_path.to_string_lossy()
            ));
        }
        if !key_pair_path.exists() {
            issues.push(format!(
                "Keypair file '{}' does not exist",
                key_pair_path.to_string_lossy()
            ));
        }
        if !issues.is_empty() {
            return Err(anyhow::anyhow!(
                "Cannot create archive due to the following issues:\n{}",
                issues.join("\n")
            ));
        }

        let keypair = {
            let mut key_pair_file = std::fs::File::open(key_pair_path)?;
            let mut key_pair_bytes = [0u8; SIGNATURE_LENGTH];
            key_pair_file.read_exact(&mut key_pair_bytes)?;
            SigningKey::from_keypair_bytes(&key_pair_bytes)?
        };

        let config: BuilderConfig = BuilderConfig::default()
            .magic(*ARCHIVE_MAGIC)
            .keypair(keypair);

        let template = Leaf::default()
            .compress(CompressMode::Always)
            .compression_algo(CompressionAlgorithm::Brotli(9))
            .encrypt(true)
            .sign(true)
            .version(1);

        let mut builder = Builder::new().template(template);
        let mut files = Vec::new();

        let walker = WalkDir::new(ASSETS_DIR).follow_links(true).into_iter();
        for entry in walker.filter_entry(|e| !is_hidden(e)) {
            let entry = entry?;
            let path = entry.path().strip_prefix(ASSETS_DIR)?.to_slash_lossy();
            if should_skip(&entry) {
                continue;
            };
            // let id = unsafe { String::from_utf8_unchecked(smaz::compress(id.as_bytes())) };
            let id = files.len().to_string();
            builder.add(File::open(entry.path())?, id)?;
            files.push(path.to_string());
        }

        let data = Cursor::new(files.join(ASSET_FILE_INDEX_SEP).into_bytes());
        builder.add(data, ASSET_FILE_INDEX)?;

        let mut target = File::create(&archive_path)?;
        builder.dump(&mut target, &config)?;

        println!("Created archive in '{}'", archive_path.to_string_lossy());

        Ok(())
    }

    // note: probably fine unless there is a really good reason for dotfiles in an active asset set
    fn is_hidden(entry: &DirEntry) -> bool {
        entry
            .file_name()
            .to_str()
            .is_some_and(|s| s.starts_with('.'))
    }

    // todo: make this configurable
    fn should_skip(entry: &DirEntry) -> bool {
        const SKIP_EXACT: &[&str] = &[".git", "node_modules", "target"];
        const SKIP_EXTENSIONS: &[&str] = &[".xcf"];

        entry.file_type().is_dir()
            || entry.file_name().to_str().is_some_and(|s| {
                SKIP_EXACT.contains(&s) || SKIP_EXTENSIONS.iter().any(|&skip| s.ends_with(skip))
            })
    }
}

mod check_files {
    use crate::GlobalArgs;
    use bevy_vach_assets::{
        vach::{
            self,
            archive::{Archive, ArchiveConfig},
            crypto::VerifyingKey,
        },
        ARCHIVE_MAGIC, ASSETS_ARCHIVE, ASSET_FILE_INDEX, ASSET_FILE_INDEX_SEP, SECRETS_PUBLIC_KEY,
    };
    use std::{env::current_dir, fs::File, io::Read};

    pub(crate) fn run(globals: &GlobalArgs) -> anyhow::Result<()> {
        let dir = current_dir()?;
        let archive_path = dir.join(&globals.assets_archive_dir).join(ASSETS_ARCHIVE);
        let public_key_path = dir.join(&globals.secrets_dir).join(SECRETS_PUBLIC_KEY);
        let mut issues = Vec::new();

        if !archive_path.exists() {
            issues.push(format!(
                "Archive file '{}' not found",
                archive_path.to_string_lossy()
            ));
        }
        if !public_key_path.exists() {
            issues.push(format!(
                "Public key file '{}' not found",
                public_key_path.to_string_lossy()
            ));
        }
        if !issues.is_empty() {
            return Err(anyhow::anyhow!(
                "Cannot check archive due to the following issues:\n{}",
                issues.join("\n")
            ));
        }

        let mut public_key_file = std::fs::File::open(public_key_path)?;
        let mut public_key_bytes = [0u8; vach::PUBLIC_KEY_LENGTH];
        public_key_file.read_exact(&mut public_key_bytes)?;
        let public_key = VerifyingKey::from_bytes(&public_key_bytes)?;

        let config = ArchiveConfig::default()
            .magic(*ARCHIVE_MAGIC)
            .key(public_key);
        let target = File::open(archive_path)?;
        let mut archive = Archive::with_config(target, &config)?;

        let file_index = archive.fetch_mut(ASSET_FILE_INDEX)?;
        let files = String::from_utf8_lossy(&file_index.data);
        let files = files.split(ASSET_FILE_INDEX_SEP).collect::<Vec<_>>();

        println!("Files in archive:");
        for (i, file) in files.iter().enumerate() {
            let f = archive.fetch_mut(&i.to_string())?;
            println!("-> {} [{}]", file, f.data.len());
        }

        Ok(())
    }
}

mod generate {
    use crate::GlobalArgs;
    use bevy_vach_assets::{vach, SECRETS_KEY_PAIR, SECRETS_PRIVATE_KEY, SECRETS_PUBLIC_KEY};
    use std::{env::current_dir, io::Write};

    pub(crate) fn run(globals: &GlobalArgs) -> anyhow::Result<()> {
        let dir = current_dir()?;
        let secrets_dir = dir.join(&globals.secrets_dir);

        let mut issues = Vec::new();
        if secrets_dir.exists() {
            issues.push(format!(
                "Secrets directory '{}' present, cannot continue (no overwrite allowed)",
                secrets_dir.to_string_lossy()
            ));
        }
        if !issues.is_empty() {
            return Err(anyhow::anyhow!(
                "Cannot generate secrets due to the following issues:\n{}",
                issues.join("\n")
            ));
        }

        std::fs::create_dir_all(&secrets_dir)?;

        let keypair = vach::crypto_utils::gen_keypair();

        let mut public_key_file = std::fs::File::create(secrets_dir.join(SECRETS_PUBLIC_KEY))?;
        public_key_file.write_all(&keypair.verifying_key().to_bytes())?;

        let mut secret_key_file = std::fs::File::create(secrets_dir.join(SECRETS_PRIVATE_KEY))?;
        secret_key_file.write_all(&keypair.to_bytes())?;

        let mut key_pair_file = std::fs::File::create(secrets_dir.join(SECRETS_KEY_PAIR))?;
        key_pair_file.write_all(&keypair.to_keypair_bytes())?;

        println!("Generated keys in '{}'", secrets_dir.to_string_lossy());
        Ok(())
    }
}
