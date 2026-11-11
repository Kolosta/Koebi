//! Settings > Language: the language of the interface, among English and the catalogs Koebi
//! ships, each with how much of the interface it translates; or the system's.

use koebi_core::{Rect, Vec2};
use koebi_i18n::{Language, tr};
use koebi_ui::{Align, Button, ButtonGroup, GroupCell, Ui, WidgetId};

use super::Stack;
use crate::language::{Choice, LanguageSetting};

const ID: WidgetId = WidgetId::new("settings-language");
/// Most choices the page lists: the system's and fifteen languages.
const CHOICES: usize = 16;

/// The Language page's state: the labels of its choices, written once per language.
#[derive(Debug, Default)]
pub(super) struct LanguagePage {
    /// One per choice: the system's, then each language.
    labels: Vec<String>,
    /// The language the labels are written in (`koebi_i18n::revision`), and the system's.
    written: Option<(u64, &'static str)>,
}

/// A language's label: its name in itself, in English, and its completion.
fn label_of(language: &Language) -> String {
    let english = language.english_name();
    let name = if english == language.name() {
        language.name().to_owned()
    } else {
        format!("{} ({english})", language.name())
    };
    if language.code() == "en" {
        return name;
    }
    koebi_i18n::fill(
        tr("{language}: {percent} % translated"),
        &[("language", &name), ("percent", &language.percent())],
    )
}

impl LanguagePage {
    pub(super) fn new() -> Self {
        Self::default()
    }

    fn write_labels(&mut self, setting: &LanguageSetting) {
        let system = setting.system_language();
        let written = (koebi_i18n::revision(), system.code());
        if self.written == Some(written) {
            return;
        }
        self.labels.clear();
        self.labels.push(koebi_i18n::fill(
            tr("The system's: {language}"),
            &[("language", &system.name())],
        ));
        self.labels
            .extend(koebi_i18n::languages().iter().map(label_of));
        self.written = Some(written);
    }

    pub(super) fn show(
        &mut self,
        ui: &mut Ui<'_>,
        stack: &mut Stack,
        setting: &mut LanguageSetting,
    ) {
        self.write_labels(setting);
        let row = ui.styles().button.height;
        let gap = stack.gap;
        let title = Rect::from_min_max(
            Vec2::new(stack.left, stack.y),
            Vec2::new(stack.right, stack.y + row),
        );
        ui.section_title(title, tr("Language of the interface"));
        stack.y += row + gap;
        let font = ui.styles().text.body.clone();
        let color = ui.styles().text.secondary;
        let text = Rect::from_min_max(
            Vec2::new(stack.left, stack.y),
            Vec2::new(stack.right, stack.y),
        );
        stack.y += ui.paragraph(
            tr("Koebi is written in English; the other languages come from translators. A message not translated yet shows in English."),
            text,
            &font,
            color,
        ) + gap;
        let languages = koebi_i18n::languages();
        // Nothing chosen yet: English, the default, shows as chosen.
        let chosen = setting
            .chosen()
            .unwrap_or(Choice::Language(koebi_i18n::english()));
        // The choices on the stack: the system's and a few languages, never many.
        let count = self.labels.len().min(CHOICES);
        let labels = &self.labels;
        let mut cells: [GroupCell<'_>; CHOICES] = core::array::from_fn(|index| {
            let selected = match index.checked_sub(1) {
                None => chosen == Choice::System,
                Some(at) => languages
                    .get(at)
                    .is_some_and(|language| chosen == Choice::Language(language)),
            };
            GroupCell::new(
                Button::new(ID.with(index as u64))
                    .label(labels.get(index).map_or("", String::as_str))
                    .align(Align::Start)
                    .selected(selected),
            )
        });
        let cells = &mut cells[..count];
        let height = row * count as f32;
        let width = (stack.right - stack.left).min(24.0 * row);
        let group =
            Rect::from_origin_size(Vec2::new(stack.left, stack.y), Vec2::new(width, height));
        let clicked = ButtonGroup::vertical().show(ui, group, cells).clicked;
        stack.y += height + gap;
        if let Some(index) = clicked {
            let choice = match index.checked_sub(1) {
                None => Some(Choice::System),
                Some(at) => languages.get(at).map(Choice::Language),
            };
            if let Some(choice) = choice {
                setting.choose(choice);
                setting.apply();
            }
        }
    }
}
