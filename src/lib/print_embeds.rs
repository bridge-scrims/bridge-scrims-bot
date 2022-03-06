//! This module allows for printing an `Embed`

use std::fmt::Display;

use serde_json::Value;
use serenity::builder::CreateEmbed;

pub struct FormatEmbed(pub CreateEmbed);

impl Display for FormatEmbed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let embed = &self.0 .0;
        if let Some(Value::Object(author)) = embed.get("author") {
            writeln!(f, "Author: {}", author["name"].as_str().unwrap())?;
            writeln!(
                f,
                "Author url: {}",
                author.get("url").map_or("No url", |x| x.as_str().unwrap())
            )?;
            writeln!(
                f,
                "Author icon: {}",
                author
                    .get("icon_url")
                    .map_or("No icon", |x| x.as_str().unwrap())
            )?;
        } else {
            writeln!(f, "Author: no author")?;
        }
        if let Some(Value::String(title)) = embed.get("title") {
            write!(f, "# {}", title)?;
        } else {
            write!(f, "# No title")?;
        }
        if let Some(Value::String(url)) = embed.get("url") {
            write!(f, " ({})", url)?;
        }
        writeln!(f)?;
        if let Some(Value::Number(color)) = embed.get("color") {
            writeln!(f, "Color: #{:6x}", color.as_u64().unwrap_or_default())?;
        }
        writeln!(f)?;
        if let Some(Value::String(desc)) = embed.get("description") {
            writeln!(f, "{}", desc)?;
        }
        writeln!(f)?;
        if let Some(Value::Array(fields)) = embed.get("fields") {
            for field in fields.iter() {
                writeln!(
                    f,
                    "- {}\n{}\n",
                    field["name"].as_str().unwrap(),
                    field["value"].as_str().unwrap()
                )?;
            }
        }
        if let Some(Value::Object(image)) = embed.get("image") {
            writeln!(f, "Image: {}", image["url"].as_str().unwrap())?;
        }
        if let Some(Value::Object(thumbnail)) = embed.get("thumbnail") {
            writeln!(f, "Thumbnail: {}", thumbnail["url"].as_str().unwrap())?;
        }
        if let Some(Value::Object(footer)) = embed.get("footer") {
            writeln!(f, "> {}", footer["text"].as_str().unwrap())?;
            if let Some(Value::String(icon)) = footer.get("icon_url") {
                writeln!(f, "Icon: {}", icon)?;
            }
        }
        if let Some(Value::String(timestamp)) = embed.get("timestamp") {
            writeln!(f, "Timestamp: {}", timestamp)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use serenity::{builder::CreateEmbed, utils::Colour};

    use super::*;

    #[test]
    fn test_big_embed() {
        let mut to_format = CreateEmbed::default();
        to_format
            .author(|author| {
                author
                    .icon_url("https://example.com")
                    .name("Bridge Scrims")
                    .url("https://example.org")
            })
            // 0xF1C40F
            .colour(Colour::GOLD)
            .description("A very nice description.")
            .fields(vec![("a", "b", false), ("c", "d", false)])
            .footer(|footer| footer.icon_url("https://example.com").text("Bridge Scrims"))
            .image("http://via.placeholder.com/150")
            .thumbnail("http://via.placeholder.com/150")
            .timestamp("1 jan 1970")
            .title("A very nice embed")
            .url("http://bridgescrims.com");

        let formatter = FormatEmbed(to_format);
        assert_eq!(
            format!("{}", formatter),
            String::from(
                "Author: Bridge Scrims
Author url: https://example.org
Author icon: https://example.com
# A very nice embed (http://bridgescrims.com)
Color: #f1c40f

A very nice description.

- a
b

- c
d

Image: http://via.placeholder.com/150
Thumbnail: http://via.placeholder.com/150
> Bridge Scrims
Icon: https://example.com
Timestamp: 1 jan 1970
"
            )
        );
    }
}
