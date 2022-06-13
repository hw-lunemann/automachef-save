# automachef-save
Automachef by HermesInteractive encrypts it's save files with the user's account ID or
a static id depending on where it was bought. Steam and Epic use their respective account IDs and Twitch uses a static ID that's the same for all users. The ID is then used to name the save directory making it possible to decrypt
any Automachef save. Transferring Automachef saves involves first decrypting the directory and then
re-encrypting. The newly decrypted/encrypted/transferred save directory will be created alongside
the original save directory.

# Usage
```
USAGE:
    automachef-save [OPTIONS] <ACTION> <Save Folder>

ARGS:
    <ACTION>
            [possible values: decrypt, encrypt, transfer]

    <Save Folder>
            e.g. '%APPDATA%/LocalLow/HermesInteractive/Automachef/Saves/<ID>'

OPTIONS:
        --epic <Epic account ID>
            Set Epic as target

        --steam <Steam ID (SteamID64)>
            Set Steam as target

        --twitch
            Set Twitch as target

        --force-overwrite
            Overwrite save files in the target directory if it already exists

    -h, --help
            Print help information

    -V, --version
            Print version information
```

# Example
```
// transfer a save to the steam user with SteamID64 38217381973127
// the new save directory will be ./test_data/38217381973127
automachef-save transfer ./test_data/1234567890 --steam 38217381973127 
```
