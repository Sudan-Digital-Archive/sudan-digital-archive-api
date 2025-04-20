//! Filter builder module for dynamic query construction across multilingual metadata tables.
//!
//! This module provides functionality to construct dynamic database filters for the digital archive search,
//! supporting multiple languages and search criteria. It's designed to be extensible for future
//! enhancements like full-text search using ts_vector indices and additional metadata fields.

use crate::models::common::MetadataLanguage;
use chrono::NaiveDateTime;
use entity::accessions_with_metadata;
use sea_orm::prelude::Expr;
use sea_orm::sea_query::{ExprTrait, Func, SimpleExpr};
use sea_orm::{sea_query, ColumnTrait};
use sea_query::extension::postgres::PgBinOper;

/// Defines the structure for filter parameters.
#[derive(Debug, Clone, Default)]
pub struct FilterParams {
    pub metadata_language: MetadataLanguage,
    pub metadata_subjects: Option<Vec<i32>>,
    pub query_term: Option<String>,
    pub date_from: Option<NaiveDateTime>,
    pub date_to: Option<NaiveDateTime>,
    pub is_private: bool,
}

/// Builds a dynamic filter expression for searching metadata across the archive.
///
/// # Arguments
///
/// * `params` - A struct containing all filter parameters
///
/// # Returns
///
/// * `Option<SimpleExpr>` - SQL expression for filtering, or None if no filters provided
///
/// The function combines these parameters to create appropriate SQL conditions based on
/// which parameters are provided, with proper language-specific handling for metadata fields.
pub fn build_filter_expression(params: FilterParams) -> Option<SimpleExpr> {
    let (title, description, lang_filter, subjects_column) = match params.metadata_language {
        MetadataLanguage::English => (
            Expr::col(accessions_with_metadata::Column::TitleEn),
            Expr::col(accessions_with_metadata::Column::DescriptionEn),
            Expr::col(accessions_with_metadata::Column::HasEnglishMetadata),
            Expr::col(accessions_with_metadata::Column::SubjectsEnIds),
        ),
        MetadataLanguage::Arabic => (
            Expr::col(accessions_with_metadata::Column::TitleAr),
            Expr::col(accessions_with_metadata::Column::DescriptionAr),
            Expr::col(accessions_with_metadata::Column::HasArabicMetadata),
            Expr::col(accessions_with_metadata::Column::SubjectsArIds),
        ),
    };

    match (
        params.query_term,
        params.date_from,
        params.date_to,
        params.metadata_subjects,
    ) {
        (Some(term), Some(from), Some(to), Some(subjects)) => {
            let query_string = format!("%{}%", term.to_lowercase());
            Some(
                Func::lower(title)
                    .like(&query_string)
                    .or(Func::lower(description).like(&query_string))
                    .and(accessions_with_metadata::Column::DublinMetadataDate.gte(from))
                    .and(accessions_with_metadata::Column::DublinMetadataDate.lte(to))
                    .and(lang_filter.eq(true))
                    .and(subjects_column.binary(PgBinOper::Overlap, subjects))
                    .and(accessions_with_metadata::Column::IsPrivate.eq(params.is_private)),
            )
        }
        (Some(term), Some(from), None, Some(subjects)) => {
            let query_string = format!("%{}%", term.to_lowercase());
            Some(
                Func::lower(title)
                    .like(&query_string)
                    .or(Func::lower(description).like(&query_string))
                    .and(accessions_with_metadata::Column::DublinMetadataDate.gte(from))
                    .and(lang_filter.eq(true))
                    .and(subjects_column.binary(PgBinOper::Overlap, subjects))
                    .and(accessions_with_metadata::Column::IsPrivate.eq(params.is_private)),
            )
        }
        (Some(term), None, Some(to), Some(subjects)) => {
            let query_string = format!("%{}%", term.to_lowercase());
            Some(
                Func::lower(title)
                    .like(&query_string)
                    .or(Func::lower(description).like(&query_string))
                    .and(accessions_with_metadata::Column::DublinMetadataDate.lte(to))
                    .and(lang_filter.eq(true))
                    .and(subjects_column.binary(PgBinOper::Overlap, subjects))
                    .and(accessions_with_metadata::Column::IsPrivate.eq(params.is_private)),
            )
        }
        (Some(term), None, None, Some(subjects)) => {
            let query_string = format!("%{}%", term.to_lowercase());
            Some(
                Func::lower(title)
                    .like(&query_string)
                    .or(Func::lower(description).like(&query_string))
                    .and(lang_filter.eq(true))
                    .and(subjects_column.binary(PgBinOper::Overlap, subjects))
                    .and(accessions_with_metadata::Column::IsPrivate.eq(params.is_private)),
            )
        }
        (None, Some(from), Some(to), Some(subjects)) => Some(
            accessions_with_metadata::Column::DublinMetadataDate
                .gte(from)
                .and(accessions_with_metadata::Column::DublinMetadataDate.lte(to))
                .and(lang_filter.eq(true))
                .and(subjects_column.binary(PgBinOper::Overlap, subjects))
                .and(accessions_with_metadata::Column::IsPrivate.eq(params.is_private)),
        ),
        (None, Some(from), None, Some(subjects)) => Some(
            accessions_with_metadata::Column::DublinMetadataDate
                .gte(from)
                .and(lang_filter.eq(true))
                .and(subjects_column.binary(PgBinOper::Overlap, subjects))
                .and(accessions_with_metadata::Column::IsPrivate.eq(params.is_private)),
        ),
        (None, None, Some(to), Some(subjects)) => Some(
            accessions_with_metadata::Column::DublinMetadataDate
                .lte(to)
                .and(lang_filter.eq(true))
                .and(subjects_column.binary(PgBinOper::Overlap, subjects))
                .and(accessions_with_metadata::Column::IsPrivate.eq(params.is_private)),
        ),
        (None, None, None, Some(subjects)) => Some(
            lang_filter
                .eq(true)
                .and(subjects_column.binary(PgBinOper::Overlap, subjects))
                .and(accessions_with_metadata::Column::IsPrivate.eq(params.is_private)),
        ),
        (Some(term), Some(from), Some(to), None) => {
            let query_string = format!("%{}%", term.to_lowercase());
            Some(
                Func::lower(title)
                    .like(&query_string)
                    .or(Func::lower(description).like(&query_string))
                    .and(accessions_with_metadata::Column::DublinMetadataDate.gte(from))
                    .and(accessions_with_metadata::Column::DublinMetadataDate.lte(to))
                    .and(lang_filter.eq(true))
                    .and(accessions_with_metadata::Column::IsPrivate.eq(params.is_private)),
            )
        }
        (Some(term), Some(from), None, None) => {
            let query_string = format!("%{}%", term.to_lowercase());
            Some(
                Func::lower(title)
                    .like(&query_string)
                    .or(Func::lower(description).like(&query_string))
                    .and(accessions_with_metadata::Column::DublinMetadataDate.gte(from))
                    .and(lang_filter.eq(true))
                    .and(accessions_with_metadata::Column::IsPrivate.eq(params.is_private)),
            )
        }
        (Some(term), None, Some(to), None) => {
            let query_string = format!("%{}%", term.to_lowercase());
            Some(
                Func::lower(title)
                    .like(&query_string)
                    .or(Func::lower(description).like(&query_string))
                    .and(accessions_with_metadata::Column::DublinMetadataDate.lte(to))
                    .and(lang_filter.eq(true))
                    .and(accessions_with_metadata::Column::IsPrivate.eq(params.is_private)),
            )
        }
        (Some(term), None, None, None) => {
            let query_string = format!("%{}%", term.to_lowercase());
            Some(
                Func::lower(title)
                    .like(&query_string)
                    .or(Func::lower(description).like(&query_string))
                    .and(lang_filter.eq(true))
                    .and(accessions_with_metadata::Column::IsPrivate.eq(params.is_private)),
            )
        }
        (None, Some(from), Some(to), None) => Some(
            accessions_with_metadata::Column::DublinMetadataDate
                .gte(from)
                .and(accessions_with_metadata::Column::DublinMetadataDate.lte(to))
                .and(lang_filter.eq(true))
                .and(accessions_with_metadata::Column::IsPrivate.eq(params.is_private)),
        ),
        (None, Some(from), None, None) => Some(
            accessions_with_metadata::Column::DublinMetadataDate
                .gte(from)
                .and(lang_filter.eq(true))
                .and(accessions_with_metadata::Column::IsPrivate.eq(params.is_private)),
        ),
        (None, None, Some(to), None) => Some(
            accessions_with_metadata::Column::DublinMetadataDate
                .lte(to)
                .and(lang_filter.eq(true))
                .and(accessions_with_metadata::Column::IsPrivate.eq(params.is_private)),
        ),
        (None, None, None, None) => Some(
            lang_filter
                .eq(true)
                .and(accessions_with_metadata::Column::IsPrivate.eq(params.is_private)),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_build_filter_none_params() {
        let params = FilterParams {
            metadata_language: MetadataLanguage::English,
            metadata_subjects: None,
            query_term: None,
            date_from: None,
            date_to: None,
            is_private: false,
        };
        let actual = build_filter_expression(params);
        let expected = Some(
            Expr::col(accessions_with_metadata::Column::HasEnglishMetadata)
                .eq(true)
                .and(accessions_with_metadata::Column::IsPrivate.eq(false)),
        );
        assert_eq!(actual, expected);
        let params = FilterParams {
            metadata_language: MetadataLanguage::Arabic,
            metadata_subjects: None,
            query_term: None,
            date_from: None,
            date_to: None,
            is_private: false,
        };
        let actual = build_filter_expression(params);
        let expected = Some(
            Expr::col(accessions_with_metadata::Column::HasArabicMetadata)
                .eq(true)
                .and(accessions_with_metadata::Column::IsPrivate.eq(false)),
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_build_filter_query_term_only() {
        let params = FilterParams {
            metadata_language: MetadataLanguage::English,
            metadata_subjects: None,
            query_term: Some("TEst".to_string()),
            date_from: None,
            date_to: None,
            is_private: false,
        };
        let actual = build_filter_expression(params);
        let (title, description) = (
            Expr::col(accessions_with_metadata::Column::TitleEn),
            Expr::col(accessions_with_metadata::Column::DescriptionEn),
        );
        let query_string = format!("%test%");
        let expected = Some(
            Func::lower(title)
                .like(&query_string)
                .or(Func::lower(description).like(&query_string))
                .and(Expr::col(accessions_with_metadata::Column::HasEnglishMetadata).eq(true))
                .and(accessions_with_metadata::Column::IsPrivate.eq(false)),
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_build_filter_arabic_query_term() {
        let params = FilterParams {
            metadata_language: MetadataLanguage::Arabic,
            metadata_subjects: None,
            query_term: Some("اختبار".to_string()),
            date_from: None,
            date_to: None,
            is_private: false,
        };
        let actual = build_filter_expression(params);
        let (title, description) = (
            Expr::col(accessions_with_metadata::Column::TitleAr),
            Expr::col(accessions_with_metadata::Column::DescriptionAr),
        );
        let query_string = format!("%اختبار%");
        let expected = Some(
            Func::lower(title)
                .like(&query_string)
                .or(Func::lower(description).like(&query_string))
                .and(Expr::col(accessions_with_metadata::Column::HasArabicMetadata).eq(true))
                .and(accessions_with_metadata::Column::IsPrivate.eq(false)),
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_build_filter_date_range() {
        let from_date = NaiveDate::from_ymd_opt(2023, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        let to_date = NaiveDate::from_ymd_opt(2023, 12, 31)
            .unwrap()
            .and_hms_opt(23, 59, 59)
            .unwrap();

        let params = FilterParams {
            metadata_language: MetadataLanguage::English,
            metadata_subjects: None,
            query_term: None,
            date_from: Some(from_date),
            date_to: Some(to_date),
            is_private: false,
        };

        let actual = build_filter_expression(params);
        let expected = Some(
            accessions_with_metadata::Column::DublinMetadataDate
                .gte(from_date)
                .and(accessions_with_metadata::Column::DublinMetadataDate.lte(to_date))
                .and(Expr::col(accessions_with_metadata::Column::HasEnglishMetadata).eq(true))
                .and(accessions_with_metadata::Column::IsPrivate.eq(false)),
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_build_filter_date_from_only() {
        let from_date = NaiveDate::from_ymd_opt(2023, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();

        let params = FilterParams {
            metadata_language: MetadataLanguage::English,
            metadata_subjects: None,
            query_term: None,
            date_from: Some(from_date),
            date_to: None,
            is_private: false,
        };

        let actual = build_filter_expression(params);
        let expected = Some(
            accessions_with_metadata::Column::DublinMetadataDate
                .gte(from_date)
                .and(Expr::col(accessions_with_metadata::Column::HasEnglishMetadata).eq(true))
                .and(accessions_with_metadata::Column::IsPrivate.eq(false)),
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_build_filter_date_to_only() {
        let to_date = NaiveDate::from_ymd_opt(2023, 12, 31)
            .unwrap()
            .and_hms_opt(23, 59, 59)
            .unwrap();

        let params = FilterParams {
            metadata_language: MetadataLanguage::English,
            metadata_subjects: None,
            query_term: None,
            date_from: None,
            date_to: Some(to_date),
            is_private: false,
        };

        let actual = build_filter_expression(params);
        let expected = Some(
            accessions_with_metadata::Column::DublinMetadataDate
                .lte(to_date)
                .and(Expr::col(accessions_with_metadata::Column::HasEnglishMetadata).eq(true))
                .and(accessions_with_metadata::Column::IsPrivate.eq(false)),
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_build_filter_query_and_date_range() {
        let from_date = NaiveDate::from_ymd_opt(2023, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        let to_date = NaiveDate::from_ymd_opt(2023, 12, 31)
            .unwrap()
            .and_hms_opt(23, 59, 59)
            .unwrap();

        let params = FilterParams {
            metadata_language: MetadataLanguage::English,
            metadata_subjects: None,
            query_term: Some("test".to_string()),
            date_from: Some(from_date),
            date_to: Some(to_date),
            is_private: false,
        };

        let actual = build_filter_expression(params);

        let (title, description) = (
            Expr::col(accessions_with_metadata::Column::TitleEn),
            Expr::col(accessions_with_metadata::Column::DescriptionEn),
        );
        let query_string = format!("%test%");
        let expected = Some(
            Func::lower(title)
                .like(&query_string)
                .or(Func::lower(description).like(&query_string))
                .and(accessions_with_metadata::Column::DublinMetadataDate.gte(from_date))
                .and(accessions_with_metadata::Column::DublinMetadataDate.lte(to_date))
                .and(Expr::col(accessions_with_metadata::Column::HasEnglishMetadata).eq(true))
                .and(accessions_with_metadata::Column::IsPrivate.eq(false)),
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_query_term_case_insensitive() {
        let params_lower = FilterParams {
            metadata_language: MetadataLanguage::English,
            metadata_subjects: None,
            query_term: Some("test".to_string()),
            date_from: None,
            date_to: None,
            is_private: false,
        };
        let actual_lower = build_filter_expression(params_lower);
        let params_upper = FilterParams {
            metadata_language: MetadataLanguage::English,
            metadata_subjects: None,
            query_term: Some("TEST".to_string()),
            date_from: None,
            date_to: None,
            is_private: false,
        };
        let actual_upper = build_filter_expression(params_upper);

        let (title, description) = (
            Expr::col(accessions_with_metadata::Column::TitleEn),
            Expr::col(accessions_with_metadata::Column::DescriptionEn),
        );
        let query_string = format!("%test%");
        let expected = Some(
            Func::lower(title)
                .like(&query_string)
                .or(Func::lower(description).like(&query_string))
                .and(Expr::col(accessions_with_metadata::Column::HasEnglishMetadata).eq(true))
                .and(accessions_with_metadata::Column::IsPrivate.eq(false)),
        );

        assert_eq!(actual_lower, expected);
        assert_eq!(actual_upper, expected);
    }

    #[test]
    fn test_build_filter_metadata_subjects_only() {
        let subjects = vec![1, 2, 3];
        let params = FilterParams {
            metadata_language: MetadataLanguage::English,
            metadata_subjects: Some(subjects.clone()),
            query_term: None,
            date_from: None,
            date_to: None,
            is_private: false,
        };
        let actual = build_filter_expression(params);

        let subjects_column = Expr::col(accessions_with_metadata::Column::SubjectsEnIds);
        let expected = Some(
            Expr::col(accessions_with_metadata::Column::HasEnglishMetadata)
                .eq(true)
                .and(subjects_column.binary(PgBinOper::Overlap, subjects))
                .and(accessions_with_metadata::Column::IsPrivate.eq(false)),
        );

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_build_filter_query_term_and_metadata_subjects() {
        let subjects = vec![1, 2, 3];
        let params = FilterParams {
            metadata_language: MetadataLanguage::English,
            metadata_subjects: Some(subjects.clone()),
            query_term: Some("test".to_string()),
            date_from: None,
            date_to: None,
            is_private: false,
        };
        let actual = build_filter_expression(params);

        let (title, description) = (
            Expr::col(accessions_with_metadata::Column::TitleEn),
            Expr::col(accessions_with_metadata::Column::DescriptionEn),
        );
        let query_string = format!("%test%");
        let subjects_column = Expr::col(accessions_with_metadata::Column::SubjectsEnIds);
        let expected = Some(
            Func::lower(title)
                .like(&query_string)
                .or(Func::lower(description).like(&query_string))
                .and(Expr::col(accessions_with_metadata::Column::HasEnglishMetadata).eq(true))
                .and(subjects_column.binary(PgBinOper::Overlap, subjects))
                .and(accessions_with_metadata::Column::IsPrivate.eq(false)),
        );

        assert_eq!(actual, expected);
    }
}
