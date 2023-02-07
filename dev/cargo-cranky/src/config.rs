use std::collections::{BTreeMap, HashMap};
use std::env::current_dir;
use std::io;

use anyhow::{bail, Result};
use cargo::util::command_prelude::ArgMatchesExt;
use serde::Deserialize;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LintLevel {
    Allow,
    Warn,
    ForceWarn,
    Deny,
    Forbid,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub(crate) struct LintCfg {
    name: String,
    level: LintLevel,
}

#[derive(Debug, Default, PartialEq, Deserialize)]
#[serde(from = "CrankyConfigMap")]
pub struct CrankyConfig {
    lints: Vec<LintCfg>,
}

fn append_for_lint_level(
    level: LintLevel,
    lints: &mut Vec<LintCfg>,
    name_prefix: &mut Vec<String>,
    map: &mut LintMap,
) {
    for (name, value) in
        map.drain_filter(|_, l| matches!(l, LintLevelOrGroup::Level(l) if *l == level))
    {
        let LintLevelOrGroup::Level(level) = value else {
            unreachable!();
        };

        name_prefix.push(name.clone());
        let name = name_prefix.join("::");
        name_prefix.pop();

        lints.push(LintCfg { name, level });
    }

    for (name, value) in map
        .iter_mut()
        .filter(|(_, v)| matches!(v, LintLevelOrGroup::Group(_)))
    {
        let LintLevelOrGroup::Group(map) = value else {
            unreachable!();
        };

        name_prefix.push(name.clone());
        append_for_lint_level(level, lints, name_prefix, map);
        name_prefix.pop();
    }

    for _ in map.drain_filter(|_, l| matches!(l, LintLevelOrGroup::Group(map) if map.is_empty())) {}
}

impl From<CrankyConfigMap> for CrankyConfig {
    fn from(mut value: CrankyConfigMap) -> Self {
        let mut lints = Vec::new();

        append_for_lint_level(
            LintLevel::Forbid,
            &mut lints,
            &mut Vec::new(),
            &mut value.map,
        );
        append_for_lint_level(LintLevel::Deny, &mut lints, &mut Vec::new(), &mut value.map);
        append_for_lint_level(
            LintLevel::ForceWarn,
            &mut lints,
            &mut Vec::new(),
            &mut value.map,
        );
        append_for_lint_level(LintLevel::Warn, &mut lints, &mut Vec::new(), &mut value.map);
        append_for_lint_level(
            LintLevel::Allow,
            &mut lints,
            &mut Vec::new(),
            &mut value.map,
        );

        assert!(value.map.is_empty());

        CrankyConfig { lints }
    }
}

type LintMap = BTreeMap<String, LintLevelOrGroup>;

#[derive(Deserialize)]
#[serde(untagged)]
enum LintLevelOrGroup {
    Level(LintLevel),
    Group(LintMap),
}

#[derive(Deserialize)]
#[serde(transparent)]
struct CrankyConfigMap {
    map: LintMap,
}

impl CrankyConfig {
    pub fn get_config() -> Result<CrankyConfig> {
        // Search for Cargo.toml in all parent directories.
        let dir = current_dir()?;

        // redirect cargo output into the void
        let cursor = io::Cursor::new(Vec::<u8>::new());
        let shell = cargo::core::Shell::from_write(Box::new(cursor));
        let Some(home_dir) = cargo::util::homedir(&dir) else {
            bail!("Could not find home directory");
        };
        let mut cargo_config = cargo::Config::new(shell, dir, home_dir);

        cargo_config.configure(0, false, None, false, false, false, &None, &[], &[])?;

        let arg_matches = cargo::util::command_prelude::ArgMatches::default();
        let mut ws = arg_matches.workspace(&cargo_config)?;

        fn get_from_custom_metadata_lints_table(
            value: &toml_edit::easy::Value,
        ) -> Result<CrankyConfig> {
            Ok(value.clone().try_into()?)
        }

        const METADATA_KEY: &str = "lints";

        fn get_from_custom_metadata(
            value: Option<&toml_edit::easy::Value>,
        ) -> Result<CrankyConfig> {
            Ok(value
                .and_then(|metadata| metadata.get(METADATA_KEY))
                .map(get_from_custom_metadata_lints_table)
                .transpose()?
                .unwrap_or_default())
        }

        let cfg = get_from_custom_metadata(ws.custom_metadata())?;

        {
            ws.load_workspace_config()?;

            for p in ws.members() {
                let _cfg_for_package = get_from_custom_metadata(p.manifest().custom_metadata())?;

                // TODO: maybe also get package-specific lints here
            }
        }

        Ok(cfg)
    }

    pub fn extra_right_args(&self) -> Vec<String> {
        let mut args = Vec::with_capacity(self.lints.len());

        for lint in &self.lints {
            let (level_head, need_space) = match lint.level {
                LintLevel::Allow => ("-A", false),
                LintLevel::Warn => ("-W", false),
                LintLevel::ForceWarn => ("--force-warn", true),
                LintLevel::Deny => ("-D", false),
                LintLevel::Forbid => ("-F", false),
            };

            match need_space {
                true => {
                    args.extend([level_head.to_string(), lint.name.clone()]);
                }
                false => {
                    args.push(format!("{}{}", level_head, lint.name));
                }
            }
        }

        args
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse_toml_1() {
        let toml_bytes = br#"
            aaa = "warn"
            bbb = "warn"
            "#;
        let config: CrankyConfig = toml::from_slice(toml_bytes).unwrap();

        assert_eq!(
            config,
            CrankyConfig {
                lints: vec![
                    LintCfg {
                        name: "aaa".into(),
                        level: LintLevel::Warn,
                    },
                    LintCfg {
                        name: "bbb".into(),
                        level: LintLevel::Warn,
                    },
                ],
            }
        )
    }

    #[test]
    fn parse_toml_2() {
        let toml_bytes = br#"
            aaa = "allow"
            bbb = "warn"
            ccc = "deny"
        "#;
        let config: CrankyConfig = toml::from_slice(toml_bytes).unwrap();

        assert_eq!(
            config,
            CrankyConfig {
                lints: vec![
                    LintCfg {
                        name: "ccc".into(),
                        level: LintLevel::Deny,
                    },
                    LintCfg {
                        name: "bbb".into(),
                        level: LintLevel::Warn,
                    },
                    LintCfg {
                        name: "aaa".into(),
                        level: LintLevel::Allow,
                    },
                ],
            }
        );

        let args = config.extra_right_args().join(" ");
        // Ordering matters! deny -> warn -> allow is the intended behavior.
        assert_eq!(args, "-Dccc -Wbbb -Aaaa");
    }
}
