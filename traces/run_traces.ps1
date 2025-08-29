Get-ChildItem $PSScriptRoot -Filter *.trace | Foreach-Object { cargo run --release $_.FullName > ($_.FullName + '.output') }
