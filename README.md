# automachef-save
automachef-transfer 0.2.0
Automachef by HermesInteractive encrypts it's save files with the user's account ID (Steam, Epic) or
a static key (Twitch). The ID is then used to name the save directory making it possible to decrypt
any regular Automachef save without supplying the ID. Transferring Automachef saves involves first
decrypting the directory and then re-encrypting. The newly decrypted/encrypted/transferred save
directory will be created alongside the original save directory.

# Usage
```
USAGE:
    automachef-save <SUBCOMMAND>

OPTIONS:
    -h, --help
            Print help information

    -V, --version
            Print version information

SUBCOMMANDS:
    decrypt
            
    encrypt
            
    help
            Print this message or the help of the given subcommand(s)
    transfer
```
## automachef-save-decrypt 
```
USAGE:
    automachef-save decrypt [OPTIONS] <Save Folder>

ARGS:
    <Save Folder>    e.g. '%APPDATA%/LocalLow/HermesInteractive/Automachef/Saves/<ID>'

OPTIONS:
        --force-overwrite    Overwrite save files in the target directory if it already exists
    -h, --help               Print help information

## automachef-save-encrypt 
USAGE:
    automachef-save encrypt [OPTIONS] <Save Folder>

ARGS:
    <Save Folder>    e.g. '%APPDATA%/LocalLow/HermesInteractive/Automachef/Saves/<ID>'

OPTIONS:
        --epic <ID>          Epic account ID
        --steam <ID>         Steam accunt ID (SteamID64)
        --twitch             Twitch
        --force-overwrite    Overwrite save files in the target directory if it already exists
    -h, --help               Print help information
```
## automachef-save-transfer 
```
USAGE:
    automachef-save transfer [OPTIONS] <Save Folder>

ARGS:
    <Save Folder>    e.g. '%APPDATA%/LocalLow/HermesInteractive/Automachef/Saves/<ID>'

OPTIONS:
        --epic <ID>          Epic account ID
        --steam <ID>         Steam accunt ID (SteamID64)
        --twitch             Twitch
        --force-overwrite    Overwrite save files in the target directory if it already exists
    -h, --help               Print help information
```
# Example
```
// produces ./test_data/1234567890.decrypted
automachef-save decrypt ./test_data/1234567890
// produces ./test_data/38217381973127
automachef-save encrypt ./test_data/1234567890.decrypted --steam 38217381973127
// produces ./test_data/38217381973127 directly
automachef-save transfer ./test_data/1234567890 --steam 38217381973127
```
