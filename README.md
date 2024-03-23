# key

```
Command Line Interface to a local or remote keepass database

Usage: key [OPTIONS] [COMMAND]

Commands:
  list  List all entries of the database
  get   Get a specific entry from the database
  set   Set the value of a specific entry in the database
  help  Print this message or the help of the given subcommand(s)

Options:
  -k, --keyfile <KEYFILE>    Path to the keyfile [env: KEEPASSDB_KEYFILE]
      --kdbx <KDBX>          Url to the keepass database file (supports file:// and s3:// schemas) [env: KEEPASSDB]
      --password <PASSWORD>  Database password [env: KEEPASSDB_PASSWORD]
  -h, --help                 Print help
  -V, --version              Print version
```
