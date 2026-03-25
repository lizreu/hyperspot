use std::fmt;

use thiserror::Error;

use crate::ast as odata_ast;

pub use crate::ast::Value as ODataValue;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldKind {
    String,
    I64,
    F64,
    Bool,
    Uuid,
    DateTimeUtc,
    Date,
    Time,
    Decimal,
}

impl fmt::Display for FieldKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FieldKind::String => write!(f, "String"),
            FieldKind::I64 => write!(f, "I64"),
            FieldKind::F64 => write!(f, "F64"),
            FieldKind::Bool => write!(f, "Bool"),
            FieldKind::Uuid => write!(f, "Uuid"),
            FieldKind::DateTimeUtc => write!(f, "DateTimeUtc"),
            FieldKind::Date => write!(f, "Date"),
            FieldKind::Time => write!(f, "Time"),
            FieldKind::Decimal => write!(f, "Decimal"),
        }
    }
}

pub trait FilterField: Copy + Eq + std::hash::Hash + fmt::Debug + 'static {
    const FIELDS: &'static [Self];

    fn name(&self) -> &'static str;

    fn kind(&self) -> FieldKind;

    fn from_name(name: &str) -> Option<Self> {
        // Try exact match first (handles both simple names and slash-delimited property paths
        // like "hierarchy/depth" if the enum defines them).
        let exact = Self::FIELDS
            .iter()
            .copied()
            .find(|f| f.name().eq_ignore_ascii_case(name));
        if exact.is_some() {
            return exact;
        }
        // Fallback: resolve by the last segment of a property path (e.g. "depth" from
        // "hierarchy/depth") so field enums that only define simple names still work.
        //
        // Note: if multiple fields share the same terminal segment this returns the
        // first match. Callers that define ambiguous field names should override
        // `from_name` with explicit slash-delimited entries.
        if let Some(last) = name.rsplit('/').next()
            && last != name
        {
            let mut iter = Self::FIELDS
                .iter()
                .copied()
                .filter(|f| f.name().eq_ignore_ascii_case(last));
            if let Some(first) = iter.next() {
                // Ambiguous: more than one field shares the same terminal segment.
                // Return None so the caller reports UnknownField instead of silently
                // picking the wrong field.
                if iter.next().is_some() {
                    return None;
                }
                return Some(first);
            }
        }
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterOp {
    Eq,
    Ne,
    Gt,
    Ge,
    Lt,
    Le,
    In,
    Contains,
    StartsWith,
    EndsWith,
    And,
    Or,
}

impl fmt::Display for FilterOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FilterOp::Eq => write!(f, "eq"),
            FilterOp::Ne => write!(f, "ne"),
            FilterOp::Gt => write!(f, "gt"),
            FilterOp::Ge => write!(f, "ge"),
            FilterOp::Lt => write!(f, "lt"),
            FilterOp::Le => write!(f, "le"),
            FilterOp::In => write!(f, "in"),
            FilterOp::Contains => write!(f, "contains"),
            FilterOp::StartsWith => write!(f, "startswith"),
            FilterOp::EndsWith => write!(f, "endswith"),
            FilterOp::And => write!(f, "and"),
            FilterOp::Or => write!(f, "or"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum FilterNode<F: FilterField> {
    Binary {
        field: F,
        op: FilterOp,
        value: ODataValue,
    },
    InList {
        field: F,
        values: Vec<ODataValue>,
    },
    Composite {
        op: FilterOp,
        children: Vec<FilterNode<F>>,
    },
    Not(Box<FilterNode<F>>),
}

impl<F: FilterField> FilterNode<F> {
    pub fn binary(field: F, op: FilterOp, value: ODataValue) -> Self {
        FilterNode::Binary { field, op, value }
    }

    #[must_use]
    pub fn and(children: Vec<FilterNode<F>>) -> Self {
        FilterNode::Composite {
            op: FilterOp::And,
            children,
        }
    }

    #[must_use]
    pub fn or(children: Vec<FilterNode<F>>) -> Self {
        FilterNode::Composite {
            op: FilterOp::Or,
            children,
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn not(inner: FilterNode<F>) -> Self {
        FilterNode::Not(Box::new(inner))
    }
}

#[derive(Debug, Error, Clone)]
#[non_exhaustive]
pub enum FilterError {
    #[error("Unknown field: {0}")]
    UnknownField(String),

    #[error("Type mismatch for field {field}: expected {expected}, got {got}")]
    TypeMismatch {
        field: String,
        expected: FieldKind,
        got: String,
    },

    #[error("Unsupported operation: {0}")]
    UnsupportedOperation(String),

    #[error("Invalid filter expression: {0}")]
    InvalidExpression(String),

    #[error("Field-to-field comparisons are not supported")]
    FieldToFieldComparison,

    #[error("Bare identifier in filter: {0}")]
    BareIdentifier(String),

    #[error("Bare literal in filter")]
    BareLiteral,
}

pub type FilterResult<T> = Result<T, FilterError>;

/// Parse an `OData` filter string into a typed `FilterNode`.
///
/// # Errors
///
/// Returns `FilterError::InvalidExpression` if parsing fails
/// or the expression cannot be converted into a typed filter node.
pub fn parse_odata_filter<F: FilterField>(raw: &str) -> FilterResult<FilterNode<F>> {
    use crate::odata_filters::parse_str;

    let ast = parse_str(raw).map_err(|e| FilterError::InvalidExpression(format!("{e}")))?;
    let ast: odata_ast::Expr = ast.into();
    convert_expr_to_filter_node::<F>(&ast)
}

/// Convert a parsed `OData` AST expression into a typed `FilterNode`.
///
/// # Errors
///
/// Returns `FilterError` if the expression is invalid, references unknown fields, uses unsupported
/// operations, or contains type mismatches.
pub fn convert_expr_to_filter_node<F: FilterField>(
    expr: &odata_ast::Expr,
) -> FilterResult<FilterNode<F>> {
    use odata_ast::Expr as E;

    match expr {
        E::And(left, right) => {
            let left_node = convert_expr_to_filter_node::<F>(left)?;
            let right_node = convert_expr_to_filter_node::<F>(right)?;
            Ok(FilterNode::and(vec![left_node, right_node]))
        }
        E::Or(left, right) => {
            let left_node = convert_expr_to_filter_node::<F>(left)?;
            let right_node = convert_expr_to_filter_node::<F>(right)?;
            Ok(FilterNode::or(vec![left_node, right_node]))
        }
        E::Not(inner) => {
            let inner_node = convert_expr_to_filter_node::<F>(inner)?;
            Ok(FilterNode::not(inner_node))
        }

        E::Compare(left, op, right) => {
            let (field_name, value) = match (&**left, &**right) {
                (E::Identifier(name), E::Value(val)) => (name.as_str(), val.clone()),
                (E::Identifier(_), E::Identifier(_)) => {
                    return Err(FilterError::FieldToFieldComparison);
                }
                _ => {
                    return Err(FilterError::InvalidExpression(
                        "Comparison must be between field and value".to_owned(),
                    ));
                }
            };

            let field = F::from_name(field_name)
                .ok_or_else(|| FilterError::UnknownField(field_name.to_owned()))?;

            validate_value_type(field, &value)?;

            let filter_op = match op {
                odata_ast::CompareOperator::Eq => FilterOp::Eq,
                odata_ast::CompareOperator::Ne => FilterOp::Ne,
                odata_ast::CompareOperator::Gt => FilterOp::Gt,
                odata_ast::CompareOperator::Ge => FilterOp::Ge,
                odata_ast::CompareOperator::Lt => FilterOp::Lt,
                odata_ast::CompareOperator::Le => FilterOp::Le,
            };

            Ok(FilterNode::binary(field, filter_op, value))
        }

        E::Function(func_name, args) => {
            let name_lower = func_name.to_ascii_lowercase();
            match (name_lower.as_str(), args.as_slice()) {
                (
                    "contains",
                    [
                        E::Identifier(field_name),
                        E::Value(odata_ast::Value::String(s)),
                    ],
                ) => {
                    let field = F::from_name(field_name)
                        .ok_or_else(|| FilterError::UnknownField(field_name.clone()))?;

                    if field.kind() != FieldKind::String {
                        return Err(FilterError::TypeMismatch {
                            field: field_name.clone(),
                            expected: FieldKind::String,
                            got: "non-string".to_owned(),
                        });
                    }

                    Ok(FilterNode::binary(
                        field,
                        FilterOp::Contains,
                        odata_ast::Value::String(s.clone()),
                    ))
                }
                (
                    "startswith",
                    [
                        E::Identifier(field_name),
                        E::Value(odata_ast::Value::String(s)),
                    ],
                ) => {
                    let field = F::from_name(field_name)
                        .ok_or_else(|| FilterError::UnknownField(field_name.clone()))?;

                    if field.kind() != FieldKind::String {
                        return Err(FilterError::TypeMismatch {
                            field: field_name.clone(),
                            expected: FieldKind::String,
                            got: "non-string".to_owned(),
                        });
                    }

                    Ok(FilterNode::binary(
                        field,
                        FilterOp::StartsWith,
                        odata_ast::Value::String(s.clone()),
                    ))
                }
                (
                    "endswith",
                    [
                        E::Identifier(field_name),
                        E::Value(odata_ast::Value::String(s)),
                    ],
                ) => {
                    let field = F::from_name(field_name)
                        .ok_or_else(|| FilterError::UnknownField(field_name.clone()))?;

                    if field.kind() != FieldKind::String {
                        return Err(FilterError::TypeMismatch {
                            field: field_name.clone(),
                            expected: FieldKind::String,
                            got: "non-string".to_owned(),
                        });
                    }

                    Ok(FilterNode::binary(
                        field,
                        FilterOp::EndsWith,
                        odata_ast::Value::String(s.clone()),
                    ))
                }
                _ => Err(FilterError::UnsupportedOperation(format!(
                    "Function '{func_name}'"
                ))),
            }
        }

        E::In(left, list) => {
            let field_name = match &**left {
                E::Identifier(name) => name.as_str(),
                _ => {
                    return Err(FilterError::InvalidExpression(
                        "IN operator requires a field identifier on the left side".to_owned(),
                    ));
                }
            };

            let field = F::from_name(field_name)
                .ok_or_else(|| FilterError::UnknownField(field_name.to_owned()))?;

            let mut values = Vec::with_capacity(list.len());
            for item in list {
                match item {
                    E::Value(val) => {
                        validate_value_type(field, val)?;
                        values.push(val.clone());
                    }
                    _ => {
                        return Err(FilterError::InvalidExpression(
                            "IN operator values must be literals".to_owned(),
                        ));
                    }
                }
            }

            if values.is_empty() {
                return Err(FilterError::InvalidExpression(
                    "IN operator requires at least one value".to_owned(),
                ));
            }

            Ok(FilterNode::InList { field, values })
        }

        E::Identifier(name) => Err(FilterError::BareIdentifier(name.clone())),
        E::Value(_) => Err(FilterError::BareLiteral),
    }
}

fn validate_value_type<F: FilterField>(field: F, value: &odata_ast::Value) -> FilterResult<()> {
    use odata_ast::Value as V;

    let kind = field.kind();
    let matches = matches!(
        (kind, value),
        (FieldKind::String, V::String(_))
            | (
                FieldKind::I64 | FieldKind::F64 | FieldKind::Decimal,
                V::Number(_)
            )
            | (FieldKind::Bool, V::Bool(_))
            | (FieldKind::Uuid, V::Uuid(_))
            | (FieldKind::DateTimeUtc, V::DateTime(_))
            | (FieldKind::Date, V::Date(_))
            | (FieldKind::Time, V::Time(_))
    );

    if matches {
        Ok(())
    } else {
        Err(FilterError::TypeMismatch {
            field: field.name().to_owned(),
            expected: kind,
            got: value.to_string(),
        })
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    // Minimal FilterField implementation for testing.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    enum TestField {
        Name,
        Age,
        Active,
    }

    impl FilterField for TestField {
        const FIELDS: &'static [Self] = &[Self::Name, Self::Age, Self::Active];

        fn name(&self) -> &'static str {
            match self {
                Self::Name => "name",
                Self::Age => "age",
                Self::Active => "active",
            }
        }

        fn kind(&self) -> FieldKind {
            match self {
                Self::Name => FieldKind::String,
                Self::Age => FieldKind::I64,
                Self::Active => FieldKind::Bool,
            }
        }
    }

    // ── FilterError variants via parse_odata_filter ───────────────────────────

    #[test]
    fn filter_error_unknown_field() {
        let result = parse_odata_filter::<TestField>("unknown_field eq 'foo'");
        assert!(
            matches!(result, Err(FilterError::UnknownField(ref f)) if f == "unknown_field"),
            "expected UnknownField, got {result:?}"
        );
    }

    #[test]
    fn filter_error_type_mismatch_string_field_with_number() {
        let result = parse_odata_filter::<TestField>("name eq 42");
        assert!(
            matches!(result, Err(FilterError::TypeMismatch { ref field, expected: FieldKind::String, .. }) if field == "name"),
            "expected TypeMismatch for 'name', got {result:?}"
        );
    }

    #[test]
    fn filter_error_type_mismatch_integer_field_with_string() {
        let result = parse_odata_filter::<TestField>("age eq 'not_a_number'");
        assert!(
            matches!(result, Err(FilterError::TypeMismatch { ref field, expected: FieldKind::I64, .. }) if field == "age"),
            "expected TypeMismatch for 'age', got {result:?}"
        );
    }

    #[test]
    fn filter_error_unsupported_operation_unknown_function() {
        let result = parse_odata_filter::<TestField>("unknown_func(name, 'foo')");
        assert!(
            matches!(result, Err(FilterError::UnsupportedOperation(_))),
            "expected UnsupportedOperation, got {result:?}"
        );
    }

    #[test]
    fn filter_error_invalid_expression_bad_syntax() {
        let result = parse_odata_filter::<TestField>("!!! invalid syntax");
        assert!(
            matches!(result, Err(FilterError::InvalidExpression(_))),
            "expected InvalidExpression, got {result:?}"
        );
    }

    #[test]
    fn filter_error_field_to_field_comparison() {
        use crate::ast::{CompareOperator, Expr};

        // Build an AST with field-to-field comparison directly (parser may not allow it)
        let ast = Expr::Compare(
            Box::new(Expr::Identifier("name".to_owned())),
            CompareOperator::Eq,
            Box::new(Expr::Identifier("active".to_owned())),
        );
        let result = convert_expr_to_filter_node::<TestField>(&ast);
        assert!(
            matches!(result, Err(FilterError::FieldToFieldComparison)),
            "expected FieldToFieldComparison, got {result:?}"
        );
    }

    #[test]
    fn filter_error_bare_identifier() {
        use crate::ast::Expr;

        let ast = Expr::Identifier("name".to_owned());
        let result = convert_expr_to_filter_node::<TestField>(&ast);
        assert!(
            matches!(result, Err(FilterError::BareIdentifier(ref s)) if s == "name"),
            "expected BareIdentifier(name), got {result:?}"
        );
    }

    #[test]
    fn filter_error_bare_literal() {
        use crate::ast::{Expr, Value};

        let ast = Expr::Value(Value::String("hello".to_owned()));
        let result = convert_expr_to_filter_node::<TestField>(&ast);
        assert!(
            matches!(result, Err(FilterError::BareLiteral)),
            "expected BareLiteral, got {result:?}"
        );
    }

    // ── FilterError Display messages ──────────────────────────────────────────

    #[test]
    fn filter_error_display_unknown_field() {
        let err = FilterError::UnknownField("foo".to_owned());
        assert_eq!(err.to_string(), "Unknown field: foo");
    }

    #[test]
    fn filter_error_display_type_mismatch() {
        let err = FilterError::TypeMismatch {
            field: "age".to_owned(),
            expected: FieldKind::I64,
            got: "'hello'".to_owned(),
        };
        assert_eq!(err.to_string(), "Type mismatch for field age: expected I64, got 'hello'");
    }

    #[test]
    fn filter_error_display_unsupported_operation() {
        let err = FilterError::UnsupportedOperation("Function 'random'".to_owned());
        assert_eq!(err.to_string(), "Unsupported operation: Function 'random'");
    }

    #[test]
    fn filter_error_display_field_to_field() {
        let err = FilterError::FieldToFieldComparison;
        assert_eq!(err.to_string(), "Field-to-field comparisons are not supported");
    }

    #[test]
    fn filter_error_display_bare_identifier() {
        let err = FilterError::BareIdentifier("name".to_owned());
        assert_eq!(err.to_string(), "Bare identifier in filter: name");
    }

    #[test]
    fn filter_error_display_bare_literal() {
        let err = FilterError::BareLiteral;
        assert_eq!(err.to_string(), "Bare literal in filter");
    }

    // ── Happy-path: parse_odata_filter succeeds ───────────────────────────────

    #[test]
    fn parse_eq_string_succeeds() {
        let result = parse_odata_filter::<TestField>("name eq 'alice'");
        assert!(result.is_ok(), "parse should succeed, got {result:?}");
        assert!(matches!(
            result.unwrap(),
            FilterNode::Binary { op: FilterOp::Eq, .. }
        ));
    }

    #[test]
    fn parse_and_succeeds() {
        let result = parse_odata_filter::<TestField>("name eq 'alice' and age eq 30");
        assert!(result.is_ok(), "parse should succeed, got {result:?}");
        assert!(matches!(
            result.unwrap(),
            FilterNode::Composite { op: FilterOp::And, .. }
        ));
    }

    #[test]
    fn parse_contains_string_field_succeeds() {
        let result = parse_odata_filter::<TestField>("contains(name, 'ali')");
        assert!(result.is_ok(), "parse should succeed, got {result:?}");
        assert!(matches!(
            result.unwrap(),
            FilterNode::Binary { op: FilterOp::Contains, .. }
        ));
    }

    #[test]
    fn parse_not_succeeds() {
        let result = parse_odata_filter::<TestField>("not (name eq 'alice')");
        assert!(result.is_ok(), "parse should succeed, got {result:?}");
        assert!(matches!(result.unwrap(), FilterNode::Not(_)));
    }

    // ── FilterError is Clone (required by modkit-odata) ───────────────────────

    #[test]
    fn filter_error_is_clone() {
        let err = FilterError::UnknownField("foo".to_owned());
        let cloned = err.clone();
        assert_eq!(err.to_string(), cloned.to_string());
    }
}
