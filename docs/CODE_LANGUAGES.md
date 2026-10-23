# Code highlighting coverage (R-CODE-01, F-19; updated V2-G / FUT-01)

Every R-CODE-01 name → its lexer. A lexer is shared only where the grammars really match.
Guard tests: `yamdv-highlight` `r_code_01_every_name_maps_to_lexer` (names → lexer) and
`tokens_cover_every_line_exactly` (every alias); the sample test named below asserts token classes.

| R-CODE-01 names | Lexer | Extra aliases | Sample test |
|---|---|---|---|
| bash, sh, shell, zsh | Bash | | `clike::shells` |
| console | **Console** (V2-G): prompt `Meta` + Bash, `\` continuation, output as text | shellsession | `clike::shells` |
| powershell, ps1 | PowerShell | pwsh, ps | `clike::shells` |
| bat, cmd | Bat | batch | `clike::shells` |
| c | C | | `clike::c_and_cpp` |
| cpp, c++, h, hpp | Cpp (C++ is a superset for headers) | cc, cxx, hxx | `clike::c_and_cpp` |
| csharp, cs | CSharp | c# | `clike::csharp_java_kotlin` |
| java | Java | | `clike::csharp_java_kotlin` |
| kotlin, kt | Kotlin | kts | `clike::csharp_java_kotlin` |
| go | Go | golang | `clike::go_python` |
| rust, rs | Rust | | `clike::rust` |
| python, py | Python | py3, python3 | `clike::go_python` |
| javascript, js, mjs, cjs | JavaScript | | `clike::js_ts` |
| jsx | **Jsx** (V2-G): JavaScript + JSX elements, attributes, text children, `{…}` | | `clike::js_ts` |
| typescript, ts | TypeScript | mts, cts | `clike::js_ts` |
| tsx | **Tsx** (V2-G): TypeScript + JSX; `<T,>` generics stay types | | `clike::js_ts` |
| json, jsonc, json5 | Json (comments, `'…'`, bare keys, `Infinity`/`NaN`) | | `misc::json` |
| yaml, yml | Yaml | | `misc::yaml` |
| toml | Toml | | `misc::toml_ini` |
| ini, conf, properties | Ini (`=`/`:`, `;`/`#`/`!` comments) | cfg, editorconfig, gitconfig | `misc::toml_ini` |
| xml, svg, xaml | Xml | plist | `markup::xml_html` |
| html, vue | **Html** (V2-G): markup + `<script>` as JavaScript + `<style>` as CSS | htm, xhtml | `markup::xml_html` |
| css, scss, less | Css (SCSS `$x`, `//`; LESS `@x` variables since V2-G) | sass | `markup::css_family` |
| sql | Sql | mysql, postgresql, psql, sqlite | `clike::sql_php_ruby` |
| markdown, md | Markdown | mdx | `markup::markdown` |
| diff, patch | Diff | udiff | `misc::diff_regex` |
| dockerfile | Dockerfile | docker, containerfile | `clike::config_languages` |
| makefile | Makefile | make, mk, mak | `clike::config_languages` |
| php / ruby, rb | Php / Ruby | | `clike::sql_php_ruby` |
| swift / lua / dart / r | Swift / Lua / Dart / R | | `clike::swift_lua_dart_r` |
| scala / perl / haskell, hs / elixir, ex | Scala / Perl / Haskell / Elixir | sc, pl, pm, exs | `clike::scala_perl_haskell_elixir` |
| nginx / graphql, gql / protobuf, proto | Nginx / GraphQl / Protobuf | nginxconf | `clike::config_languages` |
| regex | Regex | regexp, re | `misc::diff_regex` |

Lexers: 42 (V2-G adds Console, Jsx, Tsx, Html). Unmapped R-CODE-01 names: none.
Limits: a `<script>`/`<style>` start tag must close on its own line; JSX element nesting is
kept across lines up to depth 6; JSX attribute `{…}` expressions are shown as plain text;
an unterminated multi-line string/comment inside JSX resets the element stack.
Languages outside R-CODE-01 render as plain text.
