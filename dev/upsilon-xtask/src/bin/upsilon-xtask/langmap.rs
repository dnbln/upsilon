/*
 *        Copyright (c) 2023 Dinu Blanovschi
 *
 *    Licensed under the Apache License, Version 2.0 (the "License");
 *    you may not use this file except in compliance with the License.
 *    You may obtain a copy of the License at
 *
 *        https://www.apache.org/licenses/LICENSE-2.0
 *
 *    Unless required by applicable law or agreed to in writing, software
 *    distributed under the License is distributed on an "AS IS" BASIS,
 *    WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *    See the License for the specific language governing permissions and
 *    limitations under the License.
 */

use std::cell::RefCell;
use std::fmt;
use std::fmt::{Formatter, Write as _};
use std::io::Write;
use std::path::PathBuf;
use std::rc::Rc;

use anyhow::bail;
use ukonf::value::{UkonfObject, UkonfValue};
use ukonf::{Scope, UkonfConfig, UkonfFnError};
use upsilon_xtask::{ws_path, XtaskResult};

fn lang_ukonf(
    _scope: &Rc<RefCell<Scope>>,
    mut values: Vec<UkonfValue>,
) -> Result<UkonfValue, UkonfFnError> {
    if values.len() != 1 {
        bail!("lang function expects 1 argument, got {}", values.len());
    }
    let lang = values.pop().unwrap().expect_object()?;

    let mut result_lang = UkonfObject::new();

    for (k, v) in lang.into_iter() {
        match k.as_str() {
            "name" => {
                let name = v.expect_string()?;
                result_lang.insert("name".to_owned(), UkonfValue::Str(name));
            }
            "icon" => {
                let icon = v.expect_string()?;
                result_lang.insert("icon".to_owned(), UkonfValue::Str(icon));
            }
            "parent" => {
                let parent = match v {
                    UkonfValue::Null => UkonfValue::Null,
                    UkonfValue::Str(s) => UkonfValue::Str(s),
                    _ => bail!("parent must be a string or null"),
                };
                result_lang.insert("parent".to_owned(), parent);
            }
            "hljs" => {
                let hljs = v.expect_string()?;
                result_lang.insert("hljs".to_owned(), UkonfValue::Str(hljs));
            }
            "rule" => {
                let rule = v.expect_object()?;
                result_lang.insert("rule".to_owned(), UkonfValue::Object(rule));
            }
            "category" => {
                let category = v.expect_string()?;
                result_lang.insert("category".to_owned(), UkonfValue::Str(category));
            }
            k => {
                bail!("lang: unknown key {k}");
            }
        }
    }

    Ok(UkonfValue::Object(result_lang))
}

fn langmap_functions() -> ukonf::UkonfFunctions {
    let mut functions = ukonf::UkonfFunctions::new();
    functions.add_fn("lang", lang_ukonf);
    functions
}

#[derive(serde::Deserialize, Debug)]
enum LangDefRule {
    #[serde(rename = "any")]
    Any(Vec<LangDefRule>),
    #[serde(rename = "all")]
    All(Vec<LangDefRule>),
    #[serde(rename = "file_ext")]
    FileExt(Vec<String>),
    #[serde(rename = "file_name")]
    FileName(Vec<String>),
}

#[derive(serde::Deserialize, Debug)]
struct LangDef {
    name: String,
    id: String,
    parent: Option<String>,
    hljs: Option<String>,
    icon: Option<String>,
    rule: LangDefRule,
}

enum MatcherCondition {
    FileExt(String),
    FileName(String),
    PathContains(String),
}

impl fmt::Display for MatcherCondition {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            MatcherCondition::FileExt(ext) => {
                write!(f, "file_name.endsWith('{ext}')")
            }
            MatcherCondition::FileName(name) => {
                write!(f, "file_name == '{name}'")
            }
            MatcherCondition::PathContains(contains) => {
                write!(f, "file_path.contains('{contains}')")
            }
        }
    }
}

enum Matcher {
    Conjunction(Vec<Matcher>),
    Disjunction(Vec<Matcher>),
    Atom(MatcherCondition),
}

impl fmt::Display for Matcher {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Matcher::Conjunction(c) => {
                if c.is_empty() {
                    return write!(f, "true");
                }

                write!(f, "(")?;

                let mut first = true;
                for m in c {
                    if first {
                        first = false;
                    } else {
                        write!(f, " && ")?;
                    }
                    write!(f, "{m}")?;
                }

                write!(f, ")")?;

                Ok(())
            }
            Matcher::Disjunction(d) => {
                if d.is_empty() {
                    return write!(f, "false");
                }

                write!(f, "(")?;

                let mut first = true;
                for m in d {
                    if first {
                        first = false;
                    } else {
                        write!(f, " || ")?;
                    }
                    write!(f, "{m}")?;
                }

                write!(f, ")")?;

                Ok(())
            }
            Matcher::Atom(a) => {
                write!(f, "({a})")
            }
        }
    }
}

impl LangDefRule {
    fn compile_matcher(&self) -> Matcher {
        match self {
            LangDefRule::Any(rules) => {
                let mut result = Vec::new();

                for rule in rules {
                    let r = rule.compile_matcher();
                    result.push(r);
                }

                Matcher::Disjunction(result)
            }
            LangDefRule::All(rules) => {
                let mut result = Vec::new();

                for rule in rules {
                    let r = rule.compile_matcher();
                    result.push(r);
                }

                Matcher::Conjunction(result)
            }
            LangDefRule::FileExt(exts) => {
                let mut result = Vec::new();

                for ext in exts {
                    let r = Matcher::Atom(MatcherCondition::FileExt(ext.clone()));
                    result.push(r);
                }

                Matcher::Disjunction(result)
            }
            LangDefRule::FileName(names) => {
                let mut result = Vec::new();

                for name in names {
                    let r = Matcher::Atom(MatcherCondition::FileName(name.clone()));
                    result.push(r);
                }

                Matcher::Disjunction(result)
            }
        }
    }
}

#[derive(serde::Deserialize)]
#[serde(transparent)]
struct LangmapArray {
    langs: Vec<LangDef>,
}

#[derive(serde::Deserialize, Debug)]
#[serde(from = "LangmapArray")]
struct Langmap {
    langs: linked_hash_map::LinkedHashMap<String, LangDef>,
}

impl From<LangmapArray> for Langmap {
    fn from(value: LangmapArray) -> Self {
        let mut langs = linked_hash_map::LinkedHashMap::new();

        for lang in value.langs {
            langs.insert(lang.id.clone(), lang);
        }

        Langmap { langs }
    }
}

fn compile_langmap(data_path: PathBuf, target: PathBuf) -> XtaskResult<()> {
    let langmap = ukonf::UkonfRunner::new(UkonfConfig::new(vec![]), langmap_functions())
        .run(data_path)
        .expect("failed to run langmap ukonf");

    let mut new_langmap = vec![];
    for (k, v) in langmap.into_iter() {
        new_langmap.push(v.expect_object()?.with("id", k).into_value());
    }

    let langmap: Langmap = serde_json::from_value(UkonfValue::Array(new_langmap).to_json())
        .expect("failed to parse langmap");

    let mut langs = String::new();

    let header = r#"// @generated by dev/upsilon-xtask/src/bin/upsilon-xtask/langmap.rs - DO NOT EDIT
"#;

    let mut imports = String::new();

    writeln!(imports, "import type {{Lang}} from './langMap';")?;

    langs.push_str(
        r#"
const LANGS: {[keys: string]: Lang} = {
"#,
    );

    for (lang, def) in langmap.langs.iter() {
        writeln!(langs, "{lang}: {{")?;
        writeln!(langs, "id: '{lang}', ")?;
        writeln!(langs, "name: '{name}', ", name = def.name)?;
        if let Some(hljs) = &def.hljs {
            writeln!(langs, "hljs: '{hljs}', ")?;
            writeln!(langs, "hljs_def: {hljs}_hljs, ")?;

            writeln!(
                imports,
                "import {hljs}_hljs from 'svelte-highlight/languages/{hljs}';"
            )?;
        }
        if let Some(parent) = &def.parent {
            writeln!(langs, "parent: '{parent}', ")?;
        }
        if let Some(icon) = &def.icon {
            writeln!(langs, "icon: '{icon}', ")?;
        }
        let mut children = Vec::new();
        for (child_id, _) in langmap
            .langs
            .iter()
            .filter(|(k, lang_def)| lang_def.parent.as_deref() == Some(lang))
        {
            children.push(child_id);
        }

        if !children.is_empty() {
            writeln!(langs, "children: [")?;
            for child in children {
                writeln!(langs, "'{child}',")?;
            }
            writeln!(langs, "],")?;
        }

        let matcher = def.rule.compile_matcher();
        writeln!(
            langs,
            r#"matcher: (file_path: string, file_name: string) => {{
    return {matcher};
}},"#
        )?;

        writeln!(langs, "}},")?;
    }

    langs.push_str(
        r#"
};

const lookupAmongChildren = (file_path: string, file_name: string, children: string[]): Lang | null => {
	for (const child of children) {
		const lang = LANGS[child];
		if ((lang.matcher)(file_path, file_name)) {
			if (lang.children) {
				return lookupAmongChildren(file_path, file_name, lang.children) ?? lang;
			} else {
				return lang;
			}
		}
	}
	return null;
}

export const lookupLangmap = (path: string): Lang => {
	const file_path = path;
	const file_name = path.split('/').pop()!;

	return lookupAmongChildren(file_path, file_name, LANGS.text.children!) ?? LANGS.text;
};

export const lookupHljsLangImpl = (lang: Lang): any => {
	while (!lang.hljs && lang.parent) {
		lang = LANGS[lang.parent];
	}

	return lang.hljs_def;
};
"#,
    );

    let mut f = std::fs::File::create(target)?;
    f.write_all(header.as_bytes())?;
    f.write_all(imports.as_bytes())?;
    f.write_all(langs.as_bytes())?;

    Ok(())
}

pub fn run_compile_langmap_cmd() -> XtaskResult<()> {
    compile_langmap(
        ws_path!("client" / "src" / "lib" / "core" / "langMap" / "langMapData.ukonf"),
        ws_path!("client" / "src" / "lib" / "core" / "langMap" / "langMapImpl.ts"),
    )
}
