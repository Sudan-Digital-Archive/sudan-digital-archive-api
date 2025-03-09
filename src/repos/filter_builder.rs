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

/// Builds a dynamic filter expression for searching metadata across the archive.
///
/// # Arguments
///
/// * `metadata_language` - Language to search in (English or Arabic)
/// * `metadata_subjects` - Optional array of subject IDs to filter by
/// * `query_term` - Optional text to search in title and description fields
/// * `date_from` - Optional start date for filtering
/// * `date_to` - Optional end date for filtering
///
/// # Returns
///
/// * `Option<SimpleExpr>` - SQL expression for filtering, or None if no filters provided
///
/// The function combines these parameters to create appropriate SQL conditions based on
/// which parameters are provided, with proper language-specific handling for metadata fields.
pub fn build_filter_expression(
    metadata_language: MetadataLanguage,
    metadata_subjects: Option<Vec<i32>>,
    query_term: Option<String>,
    date_from: Option<NaiveDateTime>,
    date_to: Option<NaiveDateTime>,
) -> Option<SimpleExpr> {
    let (title, description, lang_filter, subjects_column) = match metadata_language {
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

    match (query_term, date_from, date_to, metadata_subjects) {
        (Some(term), Some(from), Some(to), Some(subjects)) => {
            let query_string = format!("%{}%", term.to_lowercase());
            Some(
                Func::lower(title)
                    .like(&query_string)
                    .or(Func::lower(description).like(&query_string))
                    .and(accessions_with_metadata::Column::DublinMetadataDate.gte(from))
                    .and(accessions_with_metadata::Column::DublinMetadataDate.lte(to))
                    .and(lang_filter.eq(true))
                    .and(subjects_column.binary(PgBinOper::Overlap, subjects)),
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
                    .and(subjects_column.binary(PgBinOper::Overlap, subjects)),
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
                    .and(subjects_column.binary(PgBinOper::Overlap, subjects)),
            )
        }
        (Some(term), None, None, Some(subjects)) => {
            let query_string = format!("%{}%", term.to_lowercase());
            Some(
                Func::lower(title)
                    .like(&query_string)
                    .or(Func::lower(description).like(&query_string))
                    .and(lang_filter.eq(true))
                    .and(subjects_column.binary(PgBinOper::Overlap, subjects)),
            )
        }
        (None, Some(from), Some(to), Some(subjects)) => Some(
            accessions_with_metadata::Column::DublinMetadataDate
                .gte(from)
                .and(accessions_with_metadata::Column::DublinMetadataDate.lte(to))
                .and(lang_filter.eq(true))
                .and(subjects_column.binary(PgBinOper::Overlap, subjects)),
        ),
        (None, Some(from), None, Some(subjects)) => Some(
            accessions_with_metadata::Column::DublinMetadataDate
                .gte(from)
                .and(lang_filter.eq(true))
                .and(subjects_column.binary(PgBinOper::Overlap, subjects)),
        ),
        (None, None, Some(to), Some(subjects)) => Some(
            accessions_with_metadata::Column::DublinMetadataDate
                .lte(to)
                .and(lang_filter.eq(true))
                .and(subjects_column.binary(PgBinOper::Overlap, subjects)),
        ),
        (None, None, None, Some(subjects)) => {
            Some(lang_filter.eq(true).and(subjects_column.binary(PgBinOper::Overlap, subjects)))
        }
        (Some(term), Some(from), Some(to), None) => {
            let query_string = format!("%{}%", term.to_lowercase());
            Some(
                Func::lower(title)
                    .like(&query_string)
                    .or(Func::lower(description).like(&query_string))
                    .and(accessions_with_metadata::Column::DublinMetadataDate.gte(from))
                    .and(accessions_with_metadata::Column::DublinMetadataDate.lte(to))
                    .and(lang_filter.eq(true)),
            )
        }
        (Some(term), Some(from), None, None) => {
            let query_string = format!("%{}%", term.to_lowercase());
            Some(
                Func::lower(title)
                    .like(&query_string)
                    .or(Func::lower(description).like(&query_string))
                    .and(accessions_with_metadata::Column::DublinMetadataDate.gte(from))
                    .and(lang_filter.eq(true)),
            )
        }
        (Some(term), None, Some(to), None) => {
            let query_string = format!("%{}%", term.to_lowercase());
            Some(
                Func::lower(title)
                    .like(&query_string)
                    .or(Func::lower(description).like(&query_string))
                    .and(accessions_with_metadata::Column::DublinMetadataDate.lte(to))
                    .and(lang_filter.eq(true)),
            )
        }
        (Some(term), None, None, None) => {
            let query_string = format!("%{}%", term.to_lowercase());
            Some(
                Func::lower(title)
                    .like(&query_string)
                    .or(Func::lower(description).like(&query_string))
                    .and(lang_filter.eq(true)),
            )
        }
        (None, Some(from), Some(to), None) => Some(
            accessions_with_metadata::Column::DublinMetadataDate
                .gte(from)
                .and(accessions_with_metadata::Column::DublinMetadataDate.lte(to))
                .and(lang_filter.eq(true)),
        ),
        (None, Some(from), None, None) => Some(
            accessions_with_metadata::Column::DublinMetadataDate
                .gte(from)
                .and(lang_filter.eq(true)),
        ),
        (None, None, Some(to), None) => Some(
            accessions_with_metadata::Column::DublinMetadataDate
                .lte(to)
                .and(lang_filter.eq(true)),
        ),
        (None, None, None, None) => Some(lang_filter.eq(true)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_build_filter_none_params() {
        let actual = build_filter_expression(MetadataLanguage::English, None, None, None, None);
        let expected = Some(Expr::col(accessions_with_metadata::Column::HasEnglishMetadata).eq(true));
        assert_eq!(actual, expected);
        let actual = build_filter_expression(MetadataLanguage::Arabic, None, None, None, None);
        let expected = Some(Expr::col(accessions_with_metadata::Column::HasArabicMetadata).eq(true));
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_build_filter_query_term_only() {
        let actual = build_filter_expression(
            MetadataLanguage::English,
            None,
            Some("TEst".to_string()),
            None,
            None,
        );
        let (title, description) = (
            Expr::col(accessions_with_metadata::Column::TitleEn),
            Expr::col(accessions_with_metadata::Column::DescriptionEn),
        );
        let query_string = format!("%test%");
        let expected = Some(
            Func::lower(title)
                .like(&query_string)
                .or(Func::lower(description).like(&query_string))
                .and(Expr::col(accessions_with_metadata::Column::HasEnglishMetadata).eq(true)),
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_build_filter_arabic_query_term() {
        let actual = build_filter_expression(
            MetadataLanguage::Arabic,
            None,
            Some("اختبار".to_string()),
            None,
            None,
        );
        let (title, description) = (
            Expr::col(accessions_with_metadata::Column::TitleAr),
            Expr::col(accessions_with_metadata::Column::DescriptionAr),
        );
        let query_string = format!("%اختبار%");
        let expected = Some(
            Func::lower(title)
                .like(&query_string)
                .or(Func::lower(description).like(&query_string))
                .and(Expr::col(accessions_with_metadata::Column::HasArabicMetadata).eq(true)),
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

        let actual = build_filter_expression(
            MetadataLanguage::English,
            None,
            None,
            Some(from_date),
            Some(to_date),
        );
        let expected = Some(
            accessions_with_metadata::Column::DublinMetadataDate
                .gte(from_date)
                .and(accessions_with_metadata::Column::DublinMetadataDate.lte(to_date))
                .and(Expr::col(accessions_with_metadata::Column::HasEnglishMetadata).eq(true)),
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_build_filter_date_from_only() {
        let from_date = NaiveDate::from_ymd_opt(2023, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();

        let actual =
            build_filter_expression(MetadataLanguage::English, None, None, Some(from_date), None);
        let expected = Some(
            accessions_with_metadata::Column::DublinMetadataDate
                .gte(from_date)
                .and(Expr::col(accessions_with_metadata::Column::HasEnglishMetadata).eq(true)),
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_build_filter_date_to_only() {
        let to_date = NaiveDate::from_ymd_opt(2023, 12, 31)
            .unwrap()
            .and_hms_opt(23, 59, 59)
            .unwrap();

        let actual =
            build_filter_expression(MetadataLanguage::English, None, None, None, Some(to_date));
        let expected = Some(
            accessions_with_metadata::Column::DublinMetadataDate
                .lte(to_date)
                .and(Expr::col(accessions_with_metadata::Column::HasEnglishMetadata).eq(true)),
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

        let actual = build_filter_expression(
            MetadataLanguage::English,
            None,
            Some("test".to_string()),
            Some(from_date),
            Some(to_date),
        );

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
                .and(Expr::col(accessions_with_metadata::Column::HasEnglishMetadata).eq(true)),
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_query_term_case_insensitive() {
        let actual_lower = build_filter_expression(
            MetadataLanguage::English,
            None,
            Some("test".to_string()),
            None,
            None,
        );
        let actual_upper = build_filter_expression(
            MetadataLanguage::English,
            None,
            Some("TEST".to_string()),
            None,
            None,
        );

        let (title, description) = (
            Expr::col(accessions_with_metadata::Column::TitleEn),
            Expr::col(accessions_with_metadata::Column::DescriptionEn),
        );
        let query_string = format!("%test%");
        let expected = Some(
            Func::lower(title)
                .like(&query_string)
                .or(Func::lower(description).like(&query_string))
                .and(Expr::col(accessions_with_metadata::Column::HasEnglishMetadata).eq(true)),
        );

        assert_eq!(actual_lower, expected);
        assert_eq!(actual_upper, expected);
    }

    #[test]
    fn test_build_filter_metadata_subjects_only() {
        let subjects = vec![1, 2, 3];
        let actual = build_filter_expression(
            MetadataLanguage::English,
            Some(subjects.clone()),
            None,
            None,
            None,
        );

        let subjects_column = Expr::col(accessions_with_metadata::Column::SubjectsEnIds);
        let expected = Some(
            Expr::col(accessions_with_metadata::Column::HasEnglishMetadata)
                .eq(true)
                .and(subjects_column.binary(PgBinOper::Overlap, subjects)),
        );

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_build_filter_query_term_and_metadata_subjects() {
        let subjects = vec![1, 2, 3];
        let actual = build_filter_expression(
            MetadataLanguage::English,
            Some(subjects.clone()),
            Some("test".to_string()),
            None,
            None,
        );

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
                .and(subjects_column.binary(PgBinOper::Overlap, subjects)),
        );

        assert_eq!(actual, expected);
    }
}
