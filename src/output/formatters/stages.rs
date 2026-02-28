//! Formatter for grouped stage output.

use crate::output::formatter::Formatter;
use crate::output::primitives::{write_colored, Section};
use std::io;
use termcolor::{Color, ColorSpec, WriteColor};

/// Stage entry used by the formatter.
#[derive(Debug, Clone)]
pub struct StageEntry {
    pub slug: String,
    pub name: String,
    pub difficulty: String,
    pub extension_slug: Option<String>,
    pub extension_name: Option<String>,
}

/// Formats stages grouped by base/extension sections.
pub struct StagesFormatter {
    challenge: String,
    stages: Vec<StageEntry>,
}

impl StagesFormatter {
    #[must_use] 
    pub const fn new(challenge: String, stages: Vec<StageEntry>) -> Self {
        Self { challenge, stages }
    }
}

impl Formatter for StagesFormatter {
    fn format(&self, w: &mut dyn WriteColor) -> io::Result<()> {
        Section::new(format!("Stages for {} challenge:", self.challenge)).write(w)?;

        if self.stages.is_empty() {
            write_dim(w, "  No stages found\n")?;
            return Ok(());
        }

        let mut section_letter = 'A';
        let mut current_section: Option<String> = None;
        let mut stage_num = 0usize;
        let mut base_stages_shown = false;
        let mut first_section = true;

        for stage in &self.stages {
            if let Some(ref ext_slug) = stage.extension_slug {
                // Extension section
                if current_section.as_ref() != Some(ext_slug) {
                    let ext_name = stage.extension_name.as_deref().unwrap_or(ext_slug.as_str());
                    if !first_section {
                        writeln!(w)?;
                    }
                    write_section_header(
                        w,
                        &format!("{section_letter}. {ext_name} Extension"),
                    )?;
                    section_letter = char::from_u32(section_letter as u32 + 1).unwrap_or('Z');
                    current_section = Some(ext_slug.clone());
                    stage_num = 0;
                    first_section = false;
                }
            } else if !base_stages_shown {
                // Base stages header (printed once before first base stage)
                if !first_section {
                    writeln!(w)?;
                }
                write_section_header(w, &format!("{section_letter}. Base Stages"))?;
                section_letter = char::from_u32(section_letter as u32 + 1).unwrap_or('Z');
                base_stages_shown = true;
                current_section = None;
                stage_num = 0;
                first_section = false;
            }

            stage_num += 1;

            let badge = difficulty_badge(&stage.difficulty);
            let slug_dim = format!("(#{})", stage.slug);

            // Print: "   N. Stage Name [BADGE]  (#slug)"
            write!(w, "   {stage_num}. ")?;
            write_badge(w, badge)?;
            write!(w, " {}", stage.name)?;
            write!(w, "  ")?;
            write_dim(w, &slug_dim)?;
            writeln!(w)?;
        }

        writeln!(w)?;

        // Footer summary
        write_dim(w, &format!("Total stages: {}\n", self.stages.len()))?;
        write_dim(w, "Run 'crafter test <slug>' to test a specific stage\n")?;

        Ok(())
    }
}

// ── Helpers ────────────────────────────────────────────────────────────────

fn difficulty_badge(difficulty: &str) -> &'static str {
    match difficulty {
        "very_easy" | "easy" => "[EASY]",
        "medium" => "[MED] ",
        "hard" | "very_hard" => "[HARD]",
        _ => "[    ]",
    }
}

fn badge_color(badge: &str) -> Color {
    match badge.trim() {
        "[EASY]" => Color::Green,
        "[MED]" => Color::Yellow,
        "[HARD]" => Color::Red,
        _ => Color::White,
    }
}

fn write_section_header(w: &mut dyn WriteColor, text: &str) -> io::Result<()> {
    let mut spec = ColorSpec::new();
    spec.set_bold(true);
    write_colored(w, &spec, text)?;
    writeln!(w)
}

fn write_badge(w: &mut dyn WriteColor, badge: &str) -> io::Result<()> {
    let mut spec = ColorSpec::new();
    spec.set_fg(Some(badge_color(badge))).set_bold(true);
    write_colored(w, &spec, badge)
}

fn write_dim(w: &mut dyn WriteColor, text: &str) -> io::Result<()> {
    let mut spec = ColorSpec::new();
    spec.set_dimmed(true);
    write_colored(w, &spec, text)
}
