//! Filter builder module for dynamic query construction across multilingual metadata tables.
//!
//! This module provides functionality to construct dynamic database filters for the digital archive search,
//! supporting multiple languages and search criteria. It's designed to be extensible for future
//! enhancements like full-text search using ts_vector indices and additional metadata fields.

use crate::models::common::MetadataLanguage;
use chrono::NaiveDateTime;
use entity::{accession, dublin_metadata_ar, dublin_metadata_en};
use sea_orm::prelude::Expr;
use sea_orm::sea_query::{ExprTrait, Func, SimpleExpr};
use sea_orm::ColumnTrait;

/// Builds a dynamic filter expression for searching across metadata tables based on provided criteria.
///
/// This function creates SQL filter conditions that can be applied to queries, supporting:
/// - Multilingual search across English and Arabic metadata
/// - Case-insensitive text search in titles, subjects, and descriptions
/// - Date range filtering
/// - Combination of text and date filters
///
/// # Arguments
///
/// * `metadata_language` - Language selection (English/Arabic) determining which metadata table to search
/// * `query_term` - Optional search term for text-based filtering
/// * `date_from` - Optional start date for date range filtering
/// * `date_to` - Optional end date for date range filtering
///
/// # Returns
///
/// * `Option<SimpleExpr>` - A SeaORM expression that can be used in a WHERE clause, or None if no filters applied
///
/// # Examples
///
/// ```
/// let filter = build_filter_expression(
///     MetadataLanguage::English,
///     Some("heritage".to_string()),
///     Some(start_date),
///     Some(end_date)
/// );
/// ```
///
/// # Future Enhancements
///
/// This function is designed to be extended with:
/// - Full-text search using PostgreSQL ts_vector indices
/// - Additional metadata fields for filtering
/// - More complex search patterns and combinations
/// - Support for additional languages and metadata schemas
pub fn build_filter_expression(
    metadata_language: MetadataLanguage,
    query_term: Option<String>,
    date_from: Option<NaiveDateTime>,
    date_to: Option<NaiveDateTime>,
) -> Option<SimpleExpr> {
    let (title, subject, description, lang_filter) = match metadata_language {

        MetadataLanguage::English => (
            Expr::col(dublin_metadata_en::Column::Title),
            Expr::col(dublin_metadata_en::Column::Description),
            accession::Column::DublinMetadataEn.is_not_null(),
        ),
        MetadataLanguage::Arabic => (
            Expr::col(dublin_metadata_ar::Column::Title),
            Expr::col(dublin_metadata_ar::Column::Description),
            accession::Column::DublinMetadataAr.is_not_null(),
        ),
    };

    match (query_term, date_from, date_to) {
        (Some(term), Some(from), Some(to)) => {
            let query_string = format!("%{}%", term.to_lowercase());
            Some(
                Func::lower(title)
                    .like(&query_string)
                    .or(Func::lower(description).like(&query_string))
                    .and(accession::Column::DublinMetadataDate.gte(from))
                    .and(accession::Column::DublinMetadataDate.lte(to))
                    .and(lang_filter),
            )
        }
        (Some(term), Some(from), None) => {
            let query_string = format!("%{}%", term.to_lowercase());
            Some(
                Func::lower(title)
                    .like(&query_string)
                    .or(Func::lower(description).like(&query_string))
                    .and(accession::Column::DublinMetadataDate.gte(from))
                    .and(lang_filter),
            )
        }
        (Some(term), None, Some(to)) => {
            let query_string = format!("%{}%", term.to_lowercase());
            Some(
                Func::lower(title)
                    .like(&query_string)
                    .or(Func::lower(description).like(&query_string))
                    .and(accession::Column::DublinMetadataDate.lte(to))
                    .and(lang_filter),
            )
        }
        (Some(term), None, None) => {
            let query_string = format!("%{}%", term.to_lowercase());
            Some(
                Func::lower(title)
                    .like(&query_string)
                    .or(Func::lower(subject).like(&query_string))
                    .or(Func::lower(description).like(&query_string))
                    .and(lang_filter),

            )
        }
        (None, Some(from), Some(to)) => Some(
            accession::Column::DublinMetadataDate
                .gte(from)
                .and(accession::Column::DublinMetadataDate.lte(to))
                .and(lang_filter),
        ),
        (None, Some(from), None) => Some(
            accession::Column::DublinMetadataDate
                .gte(from)
                .and(lang_filter),
        ),
        (None, None, Some(to)) => Some(
            accession::Column::DublinMetadataDate
                .lte(to)
                .and(lang_filter),
        ),
        (None, None, None) => Some(lang_filter),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_build_filter_none_params() {
        let actual = build_filter_expression(MetadataLanguage::English, None, None, None);
        assert_eq!(actual, Some(accession::Column::DublinMetadataEn.is_not_null()));
    }

    #[test]
    fn test_build_filter_query_term_only() {
        let actual = build_filter_expression(
            MetadataLanguage::English,
            Some("TEst".to_string()),
            None,
            None,
        );
        let (title, subject, description) = (
            Expr::col(dublin_metadata_en::Column::Title),
            Expr::col(dublin_metadata_en::Column::Description),
        );
        let query_string = format!("%test%");
        let expected = Some(
            Func::lower(title)
                .like(&query_string)
                .or(Func::lower(subject).like(&query_string))
                .or(Func::lower(description).like(&query_string))
                .and(accession::Column::DublinMetadataEn.is_not_null()),
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_build_filter_arabic_query_term() {
        let actual = build_filter_expression(
            MetadataLanguage::Arabic,
            Some("اختبار".to_string()),
            None,
            None,
        );
        let (title, subject, description) = (
            Expr::col(dublin_metadata_ar::Column::Title),
            Expr::col(dublin_metadata_ar::Column::Description),
        );
        let query_string = format!("%اختبار%");
        let expected = Some(
            Func::lower(title)
                .like(&query_string)
                .or(Func::lower(subject).like(&query_string))
                .or(Func::lower(description).like(&query_string))
                .and(accession::Column::DublinMetadataAr.is_not_null()),
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
            Some(from_date),
            Some(to_date),
        );
        let expected = Some(
            accession::Column::DublinMetadataDate
                .gte(from_date)
                .and(accession::Column::DublinMetadataDate.lte(to_date))
                .and(accession::Column::DublinMetadataEn.is_not_null()),
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
            build_filter_expression(MetadataLanguage::English, None, Some(from_date), None);
        let expected = Some(
            accession::Column::DublinMetadataDate
                .gte(from_date)
                .and(accession::Column::DublinMetadataEn.is_not_null()),
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_build_filter_date_to_only() {
        let to_date = NaiveDate::from_ymd_opt(2023, 12, 31)
            .unwrap()
            .and_hms_opt(23, 59, 59)
            .unwrap();

        let actual = build_filter_expression(MetadataLanguage::English, None, None, Some(to_date));
        let expected = Some(
            accession::Column::DublinMetadataDate
                .lte(to_date)
                .and(accession::Column::DublinMetadataEn.is_not_null()),
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
            Some("test".to_string()),
            Some(from_date),
            Some(to_date),
        );

        let (title, subject, description) = (
            Expr::col(dublin_metadata_en::Column::Title),
            Expr::col(dublin_metadata_en::Column::Description),
        );
        let query_string = format!("%test%");
        let expected = Some(
            Func::lower(title)
                .like(&query_string)
                .or(Func::lower(subject).like(&query_string))
                .or(Func::lower(description).like(&query_string))
                .and(accession::Column::DublinMetadataDate.gte(from_date))
                .and(accession::Column::DublinMetadataDate.lte(to_date))
                .and(accession::Column::DublinMetadataEn.is_not_null()),
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_query_term_case_insensitive() {
        let actual_lower = build_filter_expression(
            MetadataLanguage::English,
            Some("test".to_string()),
            None,
            None,
        );
        let actual_upper = build_filter_expression(
            MetadataLanguage::English,
            Some("TEST".to_string()),
            None,
            None,
        );

        let (title, description) = (
            Expr::col(dublin_metadata_en::Column::Title),
            Expr::col(dublin_metadata_en::Column::Description),
        );
        let query_string = format!("%test%");
        let expected = Some(
            Func::lower(title)
                .like(&query_string)
                .or(Func::lower(subject).like(&query_string))
                .or(Func::lower(description).like(&query_string))
                .and(accession::Column::DublinMetadataEn.is_not_null()),
        );

        assert_eq!(actual_lower, expected);
        assert_eq!(actual_upper, expected);
    }
}
