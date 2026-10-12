# Bellbird

O Bellbird é um aplicativo para GNOME que converte documentos em código LaTeX organizado. A conversão acontece localmente: o aplicativo não exige conta nem conexão com a internet para processar arquivos.

## Recursos

- Abre arquivos `.docx`, `.epub`, `.htm`, `.html`, `.markdown`, `.md`, `.odt`, `.org`, `.rst`, `.tex` e `.txt`.
- Gera documentos nos formatos ABNT (abnTeX2), IEEE (IEEEtran), ACM (acmart), Elsevier (elsarticle) e Springer Nature (sn-jnl).
- Mostra o LaTeX gerado com numeração de linhas e permite copiar ou exportar o código.
- Ao exportar, sugere a pasta Downloads e o nome do documento com a extensão `.tex`; o seletor permite escolher outro destino e nome.
- Usa os dados de autoria encontrados no documento. Quando não há autor, o LaTeX marca o local para preenchimento.
- Mantém a saída em um único arquivo `.tex`: não gera PDF nem arquivos separados de imagem ou bibliografia. Recursos de imagem e anexos são omitidos; descrições de imagem são preservadas como texto quando disponíveis.

O Bellbird não compila LaTeX. Para criar um PDF, abra o arquivo exportado em uma distribuição LaTeX com a classe e os pacotes exigidos pelo formato escolhido.

## Executar no Fedora

Requisitos: Rust 1.92 ou superior, GTK 4, libadwaita e GNU gettext. No Fedora, instale as dependências de compilação:

```sh
sudo dnf install cargo gtk4-devel libadwaita-devel gettext
```

Na primeira compilação, o Cargo precisa baixar as dependências Rust que ainda não estão no cache. Execute o projeto com:

```sh
cargo run --locked
```

Para executar as verificações do projeto:

```sh
cargo fmt --all -- --check
msgfmt --check po/pt_BR.po -o /tmp/bellbird-pt_BR.mo
msgfmt --check po/es.po -o /tmp/bellbird-es.mo
cargo clippy --all-targets -- -D warnings
cargo test
```

## Licença

GPL-3.0-or-later. Consulte [`LICENSE`](LICENSE).

## Links

- [Site do projeto](https://diegopn.github.io/bellbird-site/)
- [Repositório](https://github.com/diegopn/bellbird)
- [Relatar um problema](https://github.com/diegopn/bellbird/issues)
- [Site do criador](https://diegopn.github.io/)
