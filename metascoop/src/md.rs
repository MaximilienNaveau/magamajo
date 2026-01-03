use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use tera::{Tera, Context as TeraContext};

use crate::apps::RepoIndex;

const TABLE_START: &str = "<!-- This table is auto-generated. Do not edit -->";
const TABLE_END: &str = "<!-- end apps table -->";

const TABLE_TMPL: &str = r#"
| Icon | Name | Description | Version |
| --- | --- | --- | --- |
{% for app in apps -%}
| <a href="{{ app.sourceCode }}"><img src="fdroid/repo/icons/{{ app.packageName }}.{{ app.suggestedVersionCode }}.png" alt="{{ app.name }} icon" width="36px" height="36px"></a> | [**{{ app.name }}**]({{ app.sourceCode }}) | {{ app.summary }} | {{ app.suggestedVersionName }} ({{ app.suggestedVersionCode }}) |
{% endfor -%}
"#;

pub fn regenerate_readme(readme_path: &Path, index: &RepoIndex) -> Result<()> {
    let content = fs::read_to_string(readme_path)
        .with_context(|| format!("Failed to read README: {}", readme_path.display()))?;

    let table_start_idx = content.find(TABLE_START)
        .ok_or_else(|| anyhow::anyhow!("Cannot find table start in {:?}", readme_path))?;

    let table_end_idx = content.find(TABLE_END)
        .ok_or_else(|| anyhow::anyhow!("Cannot find table end in {:?}", readme_path))?;

    // Create Tera template
    let mut tera = Tera::default();
    tera.add_raw_template("table", TABLE_TMPL)?;

    let mut context = TeraContext::new();
    context.insert("apps", &index.apps);

    let table_content = tera.render("table", &context)?;

    // Reconstruct the file
    let mut new_content = String::new();
    new_content.push_str(&content[..table_start_idx]);
    new_content.push_str(TABLE_START);
    new_content.push_str(&table_content);
    new_content.push_str(TABLE_END);
    new_content.push_str(&content[table_end_idx + TABLE_END.len()..]);

    fs::write(readme_path, new_content)
        .with_context(|| format!("Failed to write README: {}", readme_path.display()))?;

    Ok(())
}
