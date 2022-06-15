use anyhow::Context;
use clap::{Parser, Args};
use walkdir::WalkDir;

use aes::cipher::{BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use sha1::{Digest, Sha1};

type Aes256CbcDec = cbc::Decryptor<aes::Aes256>;
type Aes256CbcEnc = cbc::Encryptor<aes::Aes256>;

type Key = [u8; 32];
type Iv = [u8; 16];

#[derive(Parser)]
#[clap(name = "automachef-transfer")]
#[clap(version = "0.2.0")]
#[clap(
    about = "Decrypt, encrypt and transfer platform and user locked Automachef save files.",
    long_about = "Automachef by HermesInteractive encrypts it's save files with the user's account ID (Steam, Epic) or a static key (Twitch). The ID is then used to name the save directory making it possible to decrypt any regular Automachef save without supplying the ID. Transferring Automachef saves involves first decrypting the directory and then re-encrypting. The newly decrypted/encrypted/transferred save directory will be created alongside the original save directory."
)]
enum Action {
    #[clap(display_order(1))]
    Decrypt {
        #[clap(flatten)]
        opts: CliOptions,
    },
    #[clap(display_order(2))]
    Encrypt { 
        #[clap(flatten)]
        platform: Platform,
        #[clap(flatten)]
        opts: CliOptions
    },
    #[clap(display_order(3))]
    Transfer { 
        #[clap(flatten)]
        platform: Platform,
        #[clap(flatten)]
        opts: CliOptions,
    }
}

impl Action {
    fn get_opts(&self) -> &CliOptions {
        match &self {
            Action::Decrypt { opts } | Action::Encrypt { opts, .. } | Action::Transfer { opts, .. } => opts,
        }
    }
}

#[derive(Args)]
struct CliOptions {
    /// Overwrite save files in the target directory if it already exists.
    #[clap(value_parser, long)]
    force_overwrite: bool,

    /// e.g. '%APPDATA%/LocalLow/HermesInteractive/Automachef/Saves/<ID>'
    #[clap(value_parser, value_name = "Save Folder")]
    input: std::path::PathBuf,
}

#[derive(Args)] 
struct Platform {
    /// Epic account ID
    #[clap(value_parser, long, value_name = "ID", display_order(0))]
    epic: Option<String>,
    /// Steam accunt ID (SteamID64)
    #[clap(value_parser, long, value_name = "ID", display_order(1))]
    steam: Option<String>,
    /// Twitch
    #[clap(value_parser, long, display_order(2))]
    twitch: bool
}

impl Platform {
    fn get_target_id(&self) -> &str {
        self.epic.as_deref().unwrap_or_else(|| self.steam.as_deref().unwrap_or("YWprc2g1NGZkaGo0MzJoMjM0amg="))
    }
}

struct EncryptedSave {
    data: Vec<u8>,
}

impl EncryptedSave {
    fn new(data: Vec<u8>) -> EncryptedSave {
        EncryptedSave { data }
    }

    fn decrypt(&self, key: &Key) -> anyhow::Result<DecryptedSave> {
        let mut base64_spec = data_encoding::BASE64.specification();
        base64_spec.ignore.push_str(" \t\r\n");
        let base = base64_spec.encoding().expect("Bug: misconfigured base64 spec");
        let mut buf = base.decode(self.text())?;

        // guaranteed to fit as len(base64) > len(cipher_text) >= len(plaintext)
        let plaintext = Aes256CbcDec::new(key.into(), self.iv().into())
            .decrypt_padded_mut::<aes::cipher::block_padding::ZeroPadding>(&mut buf)?;
        let new_len = plaintext.len();
        buf.truncate(new_len);
        Ok(DecryptedSave { text: buf })
    }

    fn _version(&self) -> &[u8] {
        &self.data[0..6]
    }

    fn iv(&self) -> &Iv {
        self.data[6..6 + 16].try_into().unwrap()
    }

    fn text(&self) -> &[u8] {
        &self.data[6 + 16..]
    }
}

struct DecryptedSave {
    text: Vec<u8>,
}

impl DecryptedSave {
    fn new(text: Vec<u8>) -> DecryptedSave {
        DecryptedSave { text }
    }

    fn encrypt(self, key: &Key, iv: Option<&Iv>) -> EncryptedSave {
        let iv = iv.unwrap_or(b"0123456789ABCDEF");

        let encrypted_buf =
            Aes256CbcEnc::new(key.into(), iv.into())
                .encrypt_padded_vec_mut::<aes::cipher::block_padding::ZeroPadding>(&self.text);

        let mut data: Vec<u8> =
            Vec::with_capacity(6 + 16 + data_encoding::BASE64.encode_len(encrypted_buf.len()));
        data.extend_from_slice(b"Ver:1\n");
        data.extend_from_slice(iv);
        data.extend_from_slice(data_encoding::BASE64.encode(&encrypted_buf).as_bytes());

        EncryptedSave::new(data)
    }
}

struct SaveDir<'a> {
    key: Option<Key>,
    dir: &'a std::path::Path,
}

fn generate_password(id: &str) -> String {
    "hewasindeedandtaforthisimpl".to_owned() + id
}

fn pbkdf1(password: &str) -> Key {
    let salt = "the salty tears provide thy nourishment";
    let iterations = 2;
    let key_len = 32;
    let mut last_hash = (password.to_owned() + salt).into_bytes();
    for _ in 0..iterations - 1 {
        last_hash = Sha1::digest(&last_hash).to_vec();
    }
    let mut bytes = Sha1::digest(&last_hash).to_vec();
    let mut ctrl = 1usize;

    while bytes.len() < key_len {
        let mut bytes2 = ctrl.to_string().into_bytes();
        bytes2.append(&mut last_hash);
        bytes.append(&mut Sha1::digest(&mut bytes2).to_vec());
        ctrl += 1;
    }

    bytes[0..key_len].try_into().unwrap()
}

fn process_entry(
    entry: walkdir::DirEntry,
    action: &Action,
    source: &SaveDir,
    target: &SaveDir,
) -> anyhow::Result<()> {
    if entry.path().is_dir() {
        return Ok(());
    }
    let data = std::fs::read(entry.path()).with_context(|| "Failed to read file.")?;

    let converted_data = match action {
        Action::Decrypt { .. } => {
            let save = EncryptedSave::new(data);
            save.decrypt(&source.key.expect("Action without source key"))
                .with_context(|| "Failed to decrypt save.")?
                .text
        }
        Action::Encrypt { .. } => {
            let save = DecryptedSave::new(data);
            save.encrypt(&target.key.expect("Encryption without target key"), None)
                .data
        }
        Action::Transfer { .. } => {
            let save = EncryptedSave::new(data);
            let decrypted = save
                .decrypt(&source.key.expect("Transfer without source key"))
                .with_context(|| "Failed to decrypt save.")?;
            decrypted
                .encrypt(
                    &target.key.expect("Transfer without source key"),
                    Some(save.iv()),
                )
                .data
        }
    };

    let target_file = entry
        .into_path()
        .iter()
        .map(|segment| {
            if segment == source.dir.file_name().expect("Bug: Action without source") {
                target.dir.file_name().expect("Bug: Action without target")
            } else {
                segment
            }
        })
        .collect::<std::path::PathBuf>();

    let parent_dir = &target_file.parent().context("Bug: Target file path is malformed")?;
    std::fs::create_dir_all(parent_dir)
            .with_context(|| format!("Failed to create dir: {}", parent_dir.display()))?;
    std::fs::write(&target_file, converted_data)
        .with_context(|| format!("Failed to write file: {}", &target_file.display()))
}

fn main() -> anyhow::Result<()> {
    let action = Action::parse();
    let opts = action.get_opts();

    if !opts.input.exists() {
        anyhow::bail!("input doesn't exist")
    }

    if !opts.input.is_dir() {
        anyhow::bail!("input has to be a directory")
    }

    if std::ffi::OsStr::new("Saves") == opts.input.file_name().unwrap() {
        anyhow::bail!("choose a subdirectory of 'Saves'")
    }

    let source_id = opts.input.file_name().and_then(|id| id.to_str()).context("the input is not a save directory")?;
    let source = SaveDir {
        key: match action {
            Action::Decrypt { .. } | Action::Transfer { .. }=> Some(pbkdf1(&generate_password(source_id))),
            Action::Encrypt { .. } => None
        },
        dir: &opts.input.to_owned(),
    };

    let mut target_dir = opts.input.to_owned();
    let target = match &action {
        Action::Decrypt { .. } => SaveDir {
            key: None,
            dir: {
                target_dir.set_extension("decrypted");
                &target_dir
            }
        },
        Action::Encrypt { platform, .. } | Action::Transfer { platform, .. } =>  SaveDir {
            key: Some(pbkdf1(&generate_password(platform.get_target_id()))),
            dir: {
                target_dir.set_file_name(platform.get_target_id());
                &target_dir
            }
        }
    };

    if !opts.force_overwrite && target.dir.exists() {
        anyhow::bail!("target directory {} already exists. Run again with '--force-overwrite' to overwrite the contents.", target.dir.display())
    }

    for entry in WalkDir::new(&source.dir).into_iter().flatten() {
        println!("{}...", entry.path().display());
        if let Err(error) = process_entry(entry, &action, &source, &target) {
            eprintln!("{}", error);
        }
    }

    Ok(())
}
