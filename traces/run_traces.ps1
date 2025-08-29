cargo build --release
Get-ChildItem $PSScriptRoot -Filter *.trace | Foreach-Object { &($PSScriptRoot + "/../target/release/cache.exe") $_.FullName '--skip-cache-desc' > ($_.FullName + '.output') }
