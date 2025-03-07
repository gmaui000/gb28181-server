set positional-arguments
group := "default"
home_dir := env_var('HOME')

# @inputs *args='':
#   bash -c 'while (( "$#" )); do echo - $1; shift; done' -- "$@"
# ----------------------- All ----------------------
alias b := build
alias r := release
alias t := test
alias c := clean
alias l := list
alias f := format

default: release
system-info:
    @echo "build platform: {{arch()}}-{{os()}}-{{os_family()}}"
    @echo "build path: {{parent_directory(justfile())}}"
list: system-info
    @just --summary --justfile {{justfile()}} --unsorted
build: system-info
    @cargo build
release: system-info
    @cargo build --release
clean:
    @cargo clean
format:
    @cargo fmt
test: system-info
    @cargo test

sanitation: system-info
    @cargo build --package file://{{parent_directory(justfile())}}/backend/sanitation --release

talkie: system-info
    @cargo build --package file://{{parent_directory(justfile())}}/backend/talkie --release