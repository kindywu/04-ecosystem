# 初始化

- sudo apt update
- sudo apt install python3-pip
- sudo apt install python3-venv

- cargo install cargo-generate
- pipx install pre-commit
- pre-commit install
- cargo install cargo-deny --locked
- cargo install typos-cli
- cargo install git-cliff
- cargo install cargo-nextest --locked

- cargo generate tyr-rust-bootcamp/template
- update master -> main
- pre-commit install
- update cliff.toml postprocessors.replace = https://github.com/kindywu/04-ecosystem

# Git

- git status
- git add .
- git commit -a -m "init" or git commit --amend
- git remote add origin https://github.com/kindywu/04-ecosystem.git
- git push -u origin main
- git commit --amend

- fix: 2024-05-28 00:32:41 [ERROR] failed to open advisory database: "/home/vscode/.cargo/advisory-db/github.com-2f857891b7f43c59"
- rm -rf /home/vscode/.cargo/advisory-db/
- git clone https://github.com/RustSec/advisory-db.git /home/vscode/.cargo/advisory-db/github.com-2f857891b7f43c59

- git tag -a v4-ecosystem-url-shortener
- git push origin v4-ecosystem-url-shortener

# sqlx macros

- windows: $env:DATABASE_URL="postgres://kindy:kindy@localhost:5432/shortener"
- linux: export DATABASE_URL=postgres://kindy:kindy@postgresql-postgresql-master-1:5432/shortener
