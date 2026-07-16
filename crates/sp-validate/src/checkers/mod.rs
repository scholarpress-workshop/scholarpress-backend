use sp_extract::document::ParsedDocument;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::LazyLock;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum Status {
    Pass,
    Fail,
    Manual,
    Error,
}

impl Status {
    pub fn as_str(&self) -> &'static str {
        match self {
            Status::Pass => "PASS",
            Status::Fail => "FAIL",
            Status::Manual => "MANUAL",
            Status::Error => "ERROR",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct EvidenceItem {
    pub page: usize,
    pub bbox: Option<(f32, f32, f32, f32)>,
    pub excerpt: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CheckResult {
    pub check_id: String,
    pub status: Status,
    pub evidence: Vec<EvidenceItem>,
    pub detail: String,
}

pub trait Checker: Send + Sync {
    fn category(&self) -> &'static str;
    fn name(&self) -> &'static str;
    fn check(&self, doc: &ParsedDocument, params: &serde_yaml::Value) -> CheckResult;
}

type CheckerFactory = fn() -> Box<dyn Checker>;

fn make_margins() -> Box<dyn Checker> {
    Box::new(crate::checkers::layout::MarginsChecker)
}

fn make_margin_symmetry() -> Box<dyn Checker> {
    Box::new(crate::checkers::layout::MarginSymmetryChecker)
}

fn make_font_size() -> Box<dyn Checker> {
    Box::new(crate::checkers::typography::FontSizeChecker)
}

fn make_font_weight() -> Box<dyn Checker> {
    Box::new(crate::checkers::typography::FontWeightChecker)
}

fn make_font_family() -> Box<dyn Checker> {
    Box::new(crate::checkers::typography::FontFamilyChecker)
}

fn make_justification() -> Box<dyn Checker> {
    Box::new(crate::checkers::typography::JustificationChecker)
}

fn make_section_presence() -> Box<dyn Checker> {
    Box::new(crate::checkers::structure::SectionPresenceChecker)
}

fn make_section_order() -> Box<dyn Checker> {
    Box::new(crate::checkers::structure::SectionOrderChecker)
}

fn make_boilerplate_match() -> Box<dyn Checker> {
    Box::new(crate::checkers::content::BoilerplateMatchChecker)
}

fn make_committee_order() -> Box<dyn Checker> {
    Box::new(crate::checkers::content::CommitteeOrderChecker)
}

fn make_toc_title_parity() -> Box<dyn Checker> {
    Box::new(crate::checkers::content::TocTitleParityChecker)
}

fn make_human_review() -> Box<dyn Checker> {
    Box::new(crate::checkers::content::HumanReviewChecker)
}

fn make_title_page_no_page_number() -> Box<dyn Checker> {
    Box::new(crate::checkers::structure::TitlePageNoPageNumberChecker)
}

fn make_acceptance_page_number() -> Box<dyn Checker> {
    Box::new(crate::checkers::structure::AcceptancePagePageNumberChecker)
}

fn make_page_numbers_format() -> Box<dyn Checker> {
    Box::new(crate::checkers::structure::PageNumbersFormatChecker)
}

fn make_headings_consistent() -> Box<dyn Checker> {
    Box::new(crate::checkers::structure::HeadingsConsistentChecker)
}

fn make_new_chapters_new_pages() -> Box<dyn Checker> {
    Box::new(crate::checkers::structure::NewChaptersNewPagesChecker)
}

fn make_hyperlinks_format() -> Box<dyn Checker> {
    Box::new(crate::checkers::structure::HyperlinksFormatChecker)
}

fn make_cv_no_page_number() -> Box<dyn Checker> {
    Box::new(crate::checkers::structure::CvNoPageNumberChecker)
}

fn make_title_page_all_caps() -> Box<dyn Checker> {
    Box::new(crate::checkers::title_page::TitlePageAllCapsChecker)
}

fn make_title_page_clause_centered() -> Box<dyn Checker> {
    Box::new(crate::checkers::title_page::TitlePageClauseCenteredChecker)
}

fn make_title_page_clause_spacing() -> Box<dyn Checker> {
    Box::new(crate::checkers::title_page::TitlePageClauseSpacingChecker)
}

fn make_copyright_page_format() -> Box<dyn Checker> {
    Box::new(crate::checkers::optional_pages::CopyrightPageFormatChecker)
}

fn make_footnotes_font() -> Box<dyn Checker> {
    Box::new(crate::checkers::footnotes::FootnotesFontChecker)
}

fn make_references_font() -> Box<dyn Checker> {
    Box::new(crate::checkers::sections::ReferencesFontChecker)
}

fn make_references_heading() -> Box<dyn Checker> {
    Box::new(crate::checkers::sections::ReferencesHeadingChecker)
}

fn make_cv_heading() -> Box<dyn Checker> {
    Box::new(crate::checkers::sections::CvHeadingChecker)
}

fn make_cv_name_position() -> Box<dyn Checker> {
    Box::new(crate::checkers::sections::CvNamePositionChecker)
}

fn make_abstract_text_centered() -> Box<dyn Checker> {
    Box::new(crate::checkers::sections::AbstractTextCenteredChecker)
}

fn make_abstract_word_count() -> Box<dyn Checker> {
    Box::new(crate::checkers::sections::AbstractWordCountChecker)
}

fn make_abstract_title_format() -> Box<dyn Checker> {
    Box::new(crate::checkers::sections::AbstractTitleFormatChecker)
}

fn make_toc_page_numbers_aligned() -> Box<dyn Checker> {
    Box::new(crate::checkers::toc_details::TocPageNumbersAlignedChecker)
}

fn make_toc_no_overhang() -> Box<dyn Checker> {
    Box::new(crate::checkers::toc_details::TocNoOverhangChecker)
}

fn make_toc_cv_no_dots() -> Box<dyn Checker> {
    Box::new(crate::checkers::toc_details::TocCvNoDotsChecker)
}

static REGISTRY: LazyLock<HashMap<(String, String), CheckerFactory>> = LazyLock::new(|| {
    let mut m = HashMap::new();
    m.insert(
        ("layout".to_string(), "margins".to_string()),
        make_margins as CheckerFactory,
    );
    m.insert(
        ("layout".to_string(), "margin_symmetry".to_string()),
        make_margin_symmetry as CheckerFactory,
    );
    m.insert(
        ("typography".to_string(), "font_size".to_string()),
        make_font_size as CheckerFactory,
    );
    m.insert(
        ("typography".to_string(), "font_weight".to_string()),
        make_font_weight as CheckerFactory,
    );
    m.insert(
        ("typography".to_string(), "font_family".to_string()),
        make_font_family as CheckerFactory,
    );
    m.insert(
        ("typography".to_string(), "justification".to_string()),
        make_justification as CheckerFactory,
    );
    m.insert(
        ("structure".to_string(), "section_presence".to_string()),
        make_section_presence as CheckerFactory,
    );
    m.insert(
        ("structure".to_string(), "section_order".to_string()),
        make_section_order as CheckerFactory,
    );
    m.insert(
        ("content".to_string(), "boilerplate_match".to_string()),
        make_boilerplate_match as CheckerFactory,
    );
    m.insert(
        ("content".to_string(), "committee_order".to_string()),
        make_committee_order as CheckerFactory,
    );
    m.insert(
        ("content".to_string(), "toc_title_parity".to_string()),
        make_toc_title_parity as CheckerFactory,
    );
    m.insert(
        ("human".to_string(), "review".to_string()),
        make_human_review as CheckerFactory,
    );
    m.insert(
        (
            "structure".to_string(),
            "title_page_no_page_number".to_string(),
        ),
        make_title_page_no_page_number as CheckerFactory,
    );
    m.insert(
        (
            "structure".to_string(),
            "acceptance_page_number".to_string(),
        ),
        make_acceptance_page_number as CheckerFactory,
    );
    m.insert(
        ("structure".to_string(), "page_numbers_format".to_string()),
        make_page_numbers_format as CheckerFactory,
    );
    m.insert(
        ("structure".to_string(), "headings_consistent".to_string()),
        make_headings_consistent as CheckerFactory,
    );
    m.insert(
        (
            "structure".to_string(),
            "new_chapters_new_pages".to_string(),
        ),
        make_new_chapters_new_pages as CheckerFactory,
    );
    m.insert(
        ("structure".to_string(), "hyperlinks_format".to_string()),
        make_hyperlinks_format as CheckerFactory,
    );
    m.insert(
        ("structure".to_string(), "cv_no_page_number".to_string()),
        make_cv_no_page_number as CheckerFactory,
    );
    m.insert(
        ("typography".to_string(), "title_page_all_caps".to_string()),
        make_title_page_all_caps as CheckerFactory,
    );
    m.insert(
        (
            "typography".to_string(),
            "title_page_clause_centered".to_string(),
        ),
        make_title_page_clause_centered as CheckerFactory,
    );
    m.insert(
        (
            "typography".to_string(),
            "title_page_clause_spacing".to_string(),
        ),
        make_title_page_clause_spacing as CheckerFactory,
    );
    m.insert(
        ("content".to_string(), "copyright_page_format".to_string()),
        make_copyright_page_format as CheckerFactory,
    );
    m.insert(
        (
            "typography".to_string(),
            "footnotes_font_consistent".to_string(),
        ),
        make_footnotes_font as CheckerFactory,
    );
    m.insert(
        (
            "typography".to_string(),
            "references_font_consistent".to_string(),
        ),
        make_references_font as CheckerFactory,
    );
    m.insert(
        (
            "structure".to_string(),
            "references_heading_format".to_string(),
        ),
        make_references_heading as CheckerFactory,
    );
    m.insert(
        ("structure".to_string(), "cv_heading_format".to_string()),
        make_cv_heading as CheckerFactory,
    );
    m.insert(
        ("structure".to_string(), "cv_name_position".to_string()),
        make_cv_name_position as CheckerFactory,
    );
    m.insert(
        (
            "typography".to_string(),
            "abstract_text_centered".to_string(),
        ),
        make_abstract_text_centered as CheckerFactory,
    );
    m.insert(
        ("content".to_string(), "abstract_word_count".to_string()),
        make_abstract_word_count as CheckerFactory,
    );
    m.insert(
        (
            "typography".to_string(),
            "abstract_title_format".to_string(),
        ),
        make_abstract_title_format as CheckerFactory,
    );
    m.insert(
        (
            "structure".to_string(),
            "toc_page_numbers_aligned".to_string(),
        ),
        make_toc_page_numbers_aligned as CheckerFactory,
    );
    m.insert(
        ("structure".to_string(), "toc_no_overhang".to_string()),
        make_toc_no_overhang as CheckerFactory,
    );
    m.insert(
        ("structure".to_string(), "toc_cv_no_dots".to_string()),
        make_toc_cv_no_dots as CheckerFactory,
    );
    m
});

pub fn get_checker(category: &str, name: &str) -> Option<Box<dyn Checker>> {
    REGISTRY
        .get(&(category.to_string(), name.to_string()))
        .map(|f| f())
}

pub mod content;
pub mod footnotes;
pub mod layout;
pub mod optional_pages;
pub mod sections;
pub mod structure;
pub mod title_page;
pub mod toc_details;
pub mod typography;
