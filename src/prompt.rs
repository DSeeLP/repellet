use std::borrow::Cow;

use reedline::Prompt;

pub static DEFAULT_PROMPT_INDICATOR: &str = "〉";
pub static DEFAULT_VI_INSERT_PROMPT_INDICATOR: &str = ": ";
pub static DEFAULT_VI_NORMAL_PROMPT_INDICATOR: &str = "〉";
pub static DEFAULT_MULTILINE_INDICATOR: &str = "::: ";

#[derive(Debug, Clone)]
pub struct StaticPrompt {
    pub left: Cow<'static, str>,
    pub indicators: StaticPromptIndicators,
    pub right: Cow<'static, str>,
}

#[derive(Debug, Clone)]
pub struct StaticPromptIndicators {
    pub default: Cow<'static, str>,
    pub vi_insert: Cow<'static, str>,
    pub vi_normal: Cow<'static, str>,
    pub multiline: Cow<'static, str>,
}

impl Default for StaticPrompt {
    fn default() -> Self {
        Self {
            left: Cow::Borrowed(""),
            indicators: StaticPromptIndicators::default(),
            right: Cow::Borrowed(""),
        }
    }
}

impl Default for StaticPromptIndicators {
    fn default() -> Self {
        Self {
            default: Cow::Borrowed(DEFAULT_PROMPT_INDICATOR),
            vi_insert: Cow::Borrowed(DEFAULT_VI_INSERT_PROMPT_INDICATOR),
            vi_normal: Cow::Borrowed(DEFAULT_VI_NORMAL_PROMPT_INDICATOR),
            multiline: Cow::Borrowed(DEFAULT_MULTILINE_INDICATOR),
        }
    }
}

impl Prompt for StaticPrompt {
    fn render_prompt_left(&self) -> Cow<str> {
        self.left.clone()
    }

    fn render_prompt_right(&self) -> Cow<str> {
        self.right.clone()
    }

    fn render_prompt_indicator(&self, prompt_mode: reedline::PromptEditMode) -> Cow<str> {
        match prompt_mode {
            reedline::PromptEditMode::Default | reedline::PromptEditMode::Emacs => {
                self.indicators.default.clone()
            }
            reedline::PromptEditMode::Vi(vi_mode) => match vi_mode {
                reedline::PromptViMode::Normal => self.indicators.vi_normal.clone(),
                reedline::PromptViMode::Insert => self.indicators.vi_insert.clone(),
            },
            reedline::PromptEditMode::Custom(str) => format!("({str})").into(),
        }
    }

    fn render_prompt_multiline_indicator(&self) -> Cow<str> {
        self.indicators.multiline.clone()
    }

    // See https://github.com/nushell/reedline/blob/3d83306b2dadd17d065d9d25ea62a36c3a0a76b2/src/prompt/default.rs#L77
    fn render_prompt_history_search_indicator(
        &self,
        history_search: reedline::PromptHistorySearch,
    ) -> Cow<str> {
        let prefix = match history_search.status {
            reedline::PromptHistorySearchStatus::Passing => "",
            reedline::PromptHistorySearchStatus::Failing => "failing ",
        };
        Cow::Owned(format!(
            "({}reverse-search: {}) ",
            prefix, history_search.term
        ))
    }
}
