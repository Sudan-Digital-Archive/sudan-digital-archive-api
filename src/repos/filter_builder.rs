//! Filter builder module for dynamic query construction across multilingual metadata tables.
//!
//! This module provides functionality to construct dynamic database filters for the digital archive search,
//! supporting multiple languages and search criteria. It's designed to be extensible for future
//! enhancements like full-text search using ts_vector indices and additional metadata fields.

use crate::models::common::MetadataLanguage;
use chrono::NaiveDateTime;
use entity::accessions_with_metadata;
use sea_orm::prelude::Expr;
use sea_orm::sea_query::SimpleExpr;
use sea_orm::{sea_query, ColumnTrait};
use sea_query::extension::postgres::PgBinOper;

/// Defines the structure for filter parameters.
#[derive(Debug, Clone, Default)]
pub struct FilterParams {
    pub metadata_language: MetadataLanguage,
    pub metadata_subjects: Option<MetadataSubjects>,
    pub query_term: Option<String>,
    pub url_filter: Option<String>,
    pub date_from: Option<NaiveDateTime>,
    pub date_to: Option<NaiveDateTime>,
    pub is_private: bool,
}

/// Defines the structure for metadata subjects filtering.
/// Easier to build match cases later of this struct than the raw format they come in.
#[derive(Debug, Clone)]
pub struct MetadataSubjects {
    pub metadata_subjects: Vec<i32>,
    pub metadata_subjects_inclusive_filter: bool,
}

/// Adds subject-based filtering to a query expression using PostgreSQL array operators.
///
/// This function enhances a `SimpleExpr` by appending conditions for filtering based on subject IDs.
/// It supports two modes of filtering:
///
/// - **Inclusive (`Overlap`):** If `metadata_subjects_inclusive_filter` is `true`, it uses the `&&`
///   operator (Overlap) to find records where the `subjects_column` array has any common elements
///   with the provided `metadata_subjects`. This is equivalent to an "any of" search.
///
/// - **Exclusive (`Contains`):** If `false`, it uses the `@>` operator (Contains) to find records
///   where the `subjects_column` array contains all of the provided `metadata_subjects`. This is
///   equivalent to an "all of" search.
///
/// # Arguments
///
/// * `expr` - The base `SimpleExpr` to which the subject filter will be added.
/// * `subjects_column` - The `Expr` representing the database column that stores subject IDs as an array.
/// * `metadata_subjects` - A `MetadataSubjects` struct containing the list of subject IDs and the filter mode.
///
/// # Returns
///
/// A new `SimpleExpr` that includes the subject filtering logic.
fn add_array_operators_to_subjects(
    expr: SimpleExpr,
    subjects_column: Expr,
    metadata_subjects: MetadataSubjects,
) -> SimpleExpr {
    if metadata_subjects.metadata_subjects_inclusive_filter {
        expr.and(subjects_column.binary(PgBinOper::Overlap, metadata_subjects.metadata_subjects))
    } else {
        expr.and(subjects_column.binary(PgBinOper::Contains, metadata_subjects.metadata_subjects))
    }
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
/// It supports full-text search, date range filtering, and subject-based filtering.
/// Note that sea orm has no ts vector datatype, so we have to get a bit funky for full text search
pub fn build_filter_expression(params: FilterParams) -> Option<SimpleExpr> {
    let (lang_filter, subjects_column) = match params.metadata_language {
        MetadataLanguage::English => (
            Expr::col(accessions_with_metadata::Column::HasEnglishMetadata),
            Expr::col(accessions_with_metadata::Column::SubjectsEnIds),
        ),
        MetadataLanguage::Arabic => (
            Expr::col(accessions_with_metadata::Column::HasArabicMetadata),
            Expr::col(accessions_with_metadata::Column::SubjectsArIds),
        ),
    };
    let (full_text_col_name, ts_lang) = match params.metadata_language {
        MetadataLanguage::English => ("full_text_en", "english"),
        MetadataLanguage::Arabic => ("full_text_ar", "arabic"),
    };

    let mut expression = match (
        params.query_term,
        params.date_from,
        params.date_to,
        params.metadata_subjects,
    ) {
        (Some(term), Some(from), Some(to), Some(metadata_subjects)) => {
            let mut expression = Expr::cust(full_text_col_name)
                .binary(
                    PgBinOper::Matches,
                    Expr::cust_with_values(format!("plainto_tsquery('{ts_lang}', $1)"), [&term]),
                )
                .and(accessions_with_metadata::Column::DublinMetadataDate.gte(from))
                .and(accessions_with_metadata::Column::DublinMetadataDate.lte(to))
                .and(lang_filter.eq(true))
                .and(accessions_with_metadata::Column::IsPrivate.eq(params.is_private));
            expression =
                add_array_operators_to_subjects(expression, subjects_column, metadata_subjects);
            Some(expression)
        }
        (Some(term), Some(from), None, Some(metadata_subjects)) => {
            let mut expression = Expr::cust(full_text_col_name)
                .binary(
                    PgBinOper::Matches,
                    Expr::cust_with_values(format!("plainto_tsquery('{ts_lang}', $1)"), [&term]),
                )
                .and(accessions_with_metadata::Column::DublinMetadataDate.gte(from))
                .and(lang_filter.eq(true))
                .and(accessions_with_metadata::Column::IsPrivate.eq(params.is_private));
            expression =
                add_array_operators_to_subjects(expression, subjects_column, metadata_subjects);

            Some(expression)
        }
        (Some(term), None, Some(to), Some(metadata_subjects)) => {
            let mut expression = Expr::cust(full_text_col_name)
                .binary(
                    PgBinOper::Matches,
                    Expr::cust_with_values(format!("plainto_tsquery('{ts_lang}', $1)"), [&term]),
                )
                .and(accessions_with_metadata::Column::DublinMetadataDate.lte(to))
                .and(lang_filter.eq(true))
                .and(accessions_with_metadata::Column::IsPrivate.eq(params.is_private));
            expression =
                add_array_operators_to_subjects(expression, subjects_column, metadata_subjects);
            Some(expression)
        }
        (Some(term), None, None, Some(metadata_subjects)) => {
            let mut expression = Expr::cust(full_text_col_name)
                .binary(
                    PgBinOper::Matches,
                    Expr::cust_with_values(format!("plainto_tsquery('{ts_lang}', $1)"), [&term]),
                )
                .and(lang_filter.eq(true))
                .and(accessions_with_metadata::Column::IsPrivate.eq(params.is_private));
            expression =
                add_array_operators_to_subjects(expression, subjects_column, metadata_subjects);
            Some(expression)
        }
        (None, Some(from), Some(to), Some(metadata_subjects)) => {
            let mut expression = accessions_with_metadata::Column::DublinMetadataDate
                .gte(from)
                .and(accessions_with_metadata::Column::DublinMetadataDate.lte(to))
                .and(lang_filter.eq(true))
                .and(accessions_with_metadata::Column::IsPrivate.eq(params.is_private));
            expression =
                add_array_operators_to_subjects(expression, subjects_column, metadata_subjects);
            Some(expression)
        }
        (None, Some(from), None, Some(metadata_subjects)) => {
            let mut expression = accessions_with_metadata::Column::DublinMetadataDate
                .gte(from)
                .and(lang_filter.eq(true))
                .and(accessions_with_metadata::Column::IsPrivate.eq(params.is_private));
            expression =
                add_array_operators_to_subjects(expression, subjects_column, metadata_subjects);
            Some(expression)
        }
        (None, None, Some(to), Some(metadata_subjects)) => {
            let mut expression = accessions_with_metadata::Column::DublinMetadataDate
                .lte(to)
                .and(lang_filter.eq(true))
                .and(accessions_with_metadata::Column::IsPrivate.eq(params.is_private));
            expression =
                add_array_operators_to_subjects(expression, subjects_column, metadata_subjects);
            Some(expression)
        }
        (None, None, None, Some(metadata_subjects)) => {
            let mut expression = lang_filter
                .eq(true)
                .and(accessions_with_metadata::Column::IsPrivate.eq(params.is_private));
            expression =
                add_array_operators_to_subjects(expression, subjects_column, metadata_subjects);
            Some(expression)
        }
        (Some(term), Some(from), Some(to), None) => Some(
            Expr::cust(full_text_col_name)
                .binary(
                    PgBinOper::Matches,
                    Expr::cust_with_values(format!("plainto_tsquery('{ts_lang}', $1)"), [&term]),
                )
                .and(accessions_with_metadata::Column::DublinMetadataDate.gte(from))
                .and(accessions_with_metadata::Column::DublinMetadataDate.lte(to))
                .and(lang_filter.eq(true))
                .and(accessions_with_metadata::Column::IsPrivate.eq(params.is_private)),
        ),
        (Some(term), Some(from), None, None) => Some(
            Expr::cust(full_text_col_name)
                .binary(
                    PgBinOper::Matches,
                    Expr::cust_with_values(format!("plainto_tsquery('{ts_lang}', $1)"), [&term]),
                )
                .and(accessions_with_metadata::Column::DublinMetadataDate.gte(from))
                .and(lang_filter.eq(true))
                .and(accessions_with_metadata::Column::IsPrivate.eq(params.is_private)),
        ),
        (Some(term), None, Some(to), None) => Some(
            Expr::cust(full_text_col_name)
                .binary(
                    PgBinOper::Matches,
                    Expr::cust_with_values(format!("plainto_tsquery('{ts_lang}', $1)"), [&term]),
                )
                .and(accessions_with_metadata::Column::DublinMetadataDate.lte(to))
                .and(lang_filter.eq(true))
                .and(accessions_with_metadata::Column::IsPrivate.eq(params.is_private)),
        ),
        (Some(term), None, None, None) => Some(
            Expr::cust(full_text_col_name)
                .binary(
                    PgBinOper::Matches,
                    Expr::cust_with_values(format!("plainto_tsquery('{ts_lang}', $1)"), [&term]),
                )
                .and(lang_filter.eq(true))
                .and(accessions_with_metadata::Column::IsPrivate.eq(params.is_private)),
        ),
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
    };

    if let Some(url) = params.url_filter {
        let url_like = format!("{}%", url);
        expression =
            expression.map(|e| e.and(accessions_with_metadata::Column::SeedUrl.like(url_like)));
    }

    expression
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_build_filter_url_filter() {
        let params = FilterParams {
            metadata_language: MetadataLanguage::English,
            metadata_subjects: None,
            query_term: None,
            url_filter: Some("https://example.com".to_string()),
            date_from: None,
            date_to: None,
            is_private: false,
        };
        let actual = build_filter_expression(params);
        let expected = Some(
            Expr::col(accessions_with_metadata::Column::HasEnglishMetadata)
                .eq(true)
                .and(accessions_with_metadata::Column::IsPrivate.eq(false))
                .and(accessions_with_metadata::Column::SeedUrl.like("https://example.com%")),
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_build_filter_none_params() {
        let params = FilterParams {
            metadata_language: MetadataLanguage::English,
            metadata_subjects: None,
            query_term: None,
            url_filter: None,
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
            url_filter: None,
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
            query_term: Some("Test".to_string()),
            url_filter: None,
            date_from: None,
            date_to: None,
            is_private: false,
        };
        let actual = build_filter_expression(params.clone());
        let (_full_text_col, ts_lang) = match params.metadata_language {
            MetadataLanguage::English => ("full_text_en", "english"),
            MetadataLanguage::Arabic => ("full_text_ar", "arabic"),
        };
        let term = "Test".to_string();

        let expected = Some(
            Expr::cust("full_text_en")
                .binary(
                    PgBinOper::Matches,
                    Expr::cust_with_values(format!("plainto_tsquery('{ts_lang}', $1)"), [&term]),
                )
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
            url_filter: None,
            date_from: None,
            date_to: None,
            is_private: false,
        };
        let actual = build_filter_expression(params.clone());
        let (_full_text_col, ts_lang) = ("full_text_ar", "arabic");
        let term = "اختبار".to_string();
        let expected = Some(
            Expr::cust("full_text_ar")
                .binary(
                    PgBinOper::Matches,
                    Expr::cust_with_values(format!("plainto_tsquery('{ts_lang}', $1)"), [&term]),
                )
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
            url_filter: None,
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
            url_filter: None,
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
            url_filter: None,
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
            url_filter: None,
            date_from: Some(from_date),
            date_to: Some(to_date),
            is_private: false,
        };

        let actual = build_filter_expression(params);

        let term = "test".to_string();
        let expected = Some(
            Expr::cust("full_text_en")
                .binary(
                    PgBinOper::Matches,
                    Expr::cust_with_values("plainto_tsquery('english', $1)", [&term]),
                )
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
            url_filter: None,
            date_from: None,
            date_to: None,
            is_private: false,
        };
        let actual_lower = build_filter_expression(params_lower);
        let params_upper = FilterParams {
            metadata_language: MetadataLanguage::English,
            metadata_subjects: None,
            query_term: Some("TEST".to_string()),
            url_filter: None,
            date_from: None,
            date_to: None,
            is_private: false,
        };
        let actual_upper = build_filter_expression(params_upper);

        let term_lower = "test".to_string();
        let expected_lower = Some(
            Expr::cust("full_text_en")
                .binary(
                    PgBinOper::Matches,
                    Expr::cust_with_values("plainto_tsquery('english', $1)", [&term_lower]),
                )
                .and(Expr::col(accessions_with_metadata::Column::HasEnglishMetadata).eq(true))
                .and(accessions_with_metadata::Column::IsPrivate.eq(false)),
        );

        let term_upper = "TEST".to_string();
        let expected_upper = Some(
            Expr::cust("full_text_en")
                .binary(
                    PgBinOper::Matches,
                    Expr::cust_with_values("plainto_tsquery('english', $1)", [&term_upper]),
                )
                .and(Expr::col(accessions_with_metadata::Column::HasEnglishMetadata).eq(true))
                .and(accessions_with_metadata::Column::IsPrivate.eq(false)),
        );

        assert_eq!(actual_lower, expected_lower);
        assert_eq!(actual_upper, expected_upper);
    }

    #[test]
    fn test_build_filter_metadata_subjects_only() {
        let subjects = vec![1, 2, 3];
        let params = FilterParams {
            metadata_language: MetadataLanguage::English,
            metadata_subjects: Some(MetadataSubjects {
                metadata_subjects: subjects.clone(),
                metadata_subjects_inclusive_filter: true,
            }),
            query_term: None,
            url_filter: None,
            date_from: None,
            date_to: None,
            is_private: false,
        };
        let actual = build_filter_expression(params);

        let subjects_column = Expr::col(accessions_with_metadata::Column::SubjectsEnIds);
        let expected = Some(
            Expr::col(accessions_with_metadata::Column::HasEnglishMetadata)
                .eq(true)
                .and(accessions_with_metadata::Column::IsPrivate.eq(false))
                .and(subjects_column.binary(PgBinOper::Overlap, subjects)),
        );

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_build_filter_metadata_subjects_exclusive() {
        let subjects = vec![1, 2, 3];
        let params = FilterParams {
            metadata_language: MetadataLanguage::English,
            metadata_subjects: Some(MetadataSubjects {
                metadata_subjects: subjects.clone(),
                metadata_subjects_inclusive_filter: false,
            }),
            query_term: None,
            url_filter: None,
            date_from: None,
            date_to: None,
            is_private: false,
        };
        let actual = build_filter_expression(params);

        let subjects_column = Expr::col(accessions_with_metadata::Column::SubjectsEnIds);
        let expected = Some(
            Expr::col(accessions_with_metadata::Column::HasEnglishMetadata)
                .eq(true)
                .and(accessions_with_metadata::Column::IsPrivate.eq(false))
                .and(subjects_column.binary(PgBinOper::Contains, subjects)),
        );

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_build_filter_query_term_and_metadata_subjects() {
        let subjects = vec![1, 2, 3];
        let params = FilterParams {
            metadata_language: MetadataLanguage::English,
            metadata_subjects: Some(MetadataSubjects {
                metadata_subjects: subjects.clone(),
                metadata_subjects_inclusive_filter: true,
            }),
            query_term: Some("test".to_string()),
            url_filter: None,
            date_from: None,
            date_to: None,
            is_private: false,
        };
        let actual = build_filter_expression(params);

        let term = "test".to_string();
        let subjects_column = Expr::col(accessions_with_metadata::Column::SubjectsEnIds);
        let expected = Some(
            Expr::cust("full_text_en")
                .binary(
                    PgBinOper::Matches,
                    Expr::cust_with_values("plainto_tsquery('english', $1)", [&term]),
                )
                .and(Expr::col(accessions_with_metadata::Column::HasEnglishMetadata).eq(true))
                .and(accessions_with_metadata::Column::IsPrivate.eq(false))
                .and(subjects_column.binary(PgBinOper::Overlap, subjects)),
        );

        assert_eq!(actual, expected);
    }
}
