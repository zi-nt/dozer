use dozer_sql_expression::{
    builder::ExpressionBuilder,
    sqlparser::ast::{Expr, FunctionArg, FunctionArgExpr, Ident, ObjectName, Value},
};
use dozer_types::{
    chrono::Duration,
    types::{FieldDefinition, Schema},
};

use crate::{
    errors::{JoinError, PipelineError, WindowError},
    pipeline_builder::from_builder::{TableOperatorArg, TableOperatorDescriptor},
};

use super::operator::WindowType;

const _ARG_SOURCE: usize = 0;
const ARG_COLUMN: usize = 1;

const ARG_TUMBLE_INTERVAL: usize = 2;

const ARG_HOP_SIZE: usize = 2;
const ARG_HOP_INTERVAL: usize = 3;

pub(crate) fn window_from_table_operator(
    operator: &TableOperatorDescriptor,
    schema: &Schema,
) -> Result<Option<WindowType>, WindowError> {
    if operator.name.to_uppercase() == "TUMBLE" {
        let column_index = get_window_column_index(&operator.args, schema)?;
        let interval_arg = operator
            .args
            .get(ARG_TUMBLE_INTERVAL)
            .ok_or(WindowError::WindowMissingIntervalArgument)?;
        let argument = if let TableOperatorArg::Argument(arg) = interval_arg {
            arg
        } else {
            return Err(WindowError::WindowInvalidInterval("".to_string()));
        };
        let interval = get_window_interval(argument)?;

        Ok(Some(WindowType::Tumble {
            column_index,
            interval,
        }))
    } else if operator.name.to_uppercase() == "HOP" {
        let column_index = get_window_column_index(&operator.args, schema)?;
        let hop_arg = operator
            .args
            .get(ARG_HOP_SIZE)
            .ok_or(WindowError::WindowMissingHopSizeArgument)?;
        let argument = if let TableOperatorArg::Argument(arg) = hop_arg {
            arg
        } else {
            return Err(WindowError::WindowInvalidHop("".to_string()));
        };
        let hop_size = get_window_hop(argument)?;
        let interval_arg = operator
            .args
            .get(ARG_HOP_INTERVAL)
            .ok_or(WindowError::WindowMissingIntervalArgument)?;
        let argument = if let TableOperatorArg::Argument(arg) = interval_arg {
            arg
        } else {
            return Err(WindowError::WindowInvalidInterval("".to_string()));
        };
        let interval = get_window_interval(argument)?;

        return Ok(Some(WindowType::Hop {
            column_index,
            hop_size,
            interval,
        }));
    } else {
        return Err(WindowError::UnsupportedRelationFunction(
            operator.name.clone(),
        ));
    }
}

fn get_window_interval(interval_arg: &FunctionArg) -> Result<Duration, WindowError> {
    match interval_arg {
        FunctionArg::Named { name, arg: _ } => {
            let column_name = ExpressionBuilder::normalize_ident(name);
            Err(WindowError::WindowInvalidInterval(column_name))
        }
        FunctionArg::Unnamed(arg_expr) => match arg_expr {
            FunctionArgExpr::Expr(expr) => match expr {
                Expr::Value(Value::SingleQuotedString(s) | Value::DoubleQuotedString(s)) => {
                    let interval: Duration = parse_duration_string(s)
                        .map_err(|_| WindowError::WindowInvalidInterval(s.to_owned()))?;
                    Ok(interval)
                }
                _ => Err(WindowError::WindowInvalidInterval(expr.to_string())),
            },
            FunctionArgExpr::QualifiedWildcard(_) => {
                Err(WindowError::WindowInvalidInterval("*".to_string()))
            }
            FunctionArgExpr::Wildcard => Err(WindowError::WindowInvalidInterval("*".to_string())),
        },
    }
}

fn get_window_hop(hop_arg: &FunctionArg) -> Result<Duration, WindowError> {
    match hop_arg {
        FunctionArg::Named { name, arg: _ } => {
            let column_name = ExpressionBuilder::normalize_ident(name);
            Err(WindowError::WindowInvalidHop(column_name))
        }
        FunctionArg::Unnamed(arg_expr) => match arg_expr {
            FunctionArgExpr::Expr(expr) => match expr {
                Expr::Value(Value::SingleQuotedString(s) | Value::DoubleQuotedString(s)) => {
                    let interval: Duration = parse_duration_string(s)
                        .map_err(|_| WindowError::WindowInvalidHop(s.to_owned()))?;
                    Ok(interval)
                }
                _ => Err(WindowError::WindowInvalidHop(expr.to_string())),
            },
            FunctionArgExpr::QualifiedWildcard(_) => {
                Err(WindowError::WindowInvalidHop("*".to_string()))
            }
            FunctionArgExpr::Wildcard => Err(WindowError::WindowInvalidHop("*".to_string())),
        },
    }
}

fn get_window_column_index(
    args: &[TableOperatorArg],
    schema: &Schema,
) -> Result<usize, WindowError> {
    let column_arg = args
        .get(ARG_COLUMN)
        .ok_or(WindowError::WindowMissingColumnArgument)?;
    let argument = if let TableOperatorArg::Argument(arg) = column_arg {
        arg
    } else {
        return Err(WindowError::WindowInvalidColumn("".to_string()));
    };
    match argument {
        FunctionArg::Named { name, arg: _ } => {
            let column_name = ExpressionBuilder::normalize_ident(name);
            Err(WindowError::WindowInvalidColumn(column_name))
        }
        FunctionArg::Unnamed(arg_expr) => match arg_expr {
            FunctionArgExpr::Expr(expr) => match expr {
                Expr::Identifier(ident) => {
                    let column_name = ExpressionBuilder::normalize_ident(ident);
                    let index = get_field_index(&[ident.clone()], schema)
                        .map_err(|_| WindowError::WindowInvalidColumn(column_name.clone()))?;

                    Ok(index.ok_or(WindowError::WindowInvalidColumn(column_name))?)
                }
                Expr::CompoundIdentifier(ident) => {
                    let column_name = ExpressionBuilder::fullname_from_ident(ident);
                    let index = get_field_index(ident, schema)
                        .map_err(|_| WindowError::WindowInvalidColumn(column_name.clone()))?;

                    Ok(index.ok_or(WindowError::WindowInvalidColumn(column_name))?)
                }
                _ => Err(WindowError::WindowInvalidColumn(expr.to_string())),
            },
            FunctionArgExpr::QualifiedWildcard(_) => {
                Err(WindowError::WindowInvalidColumn("*".to_string()))
            }
            FunctionArgExpr::Wildcard => Err(WindowError::WindowInvalidColumn("*".to_string())),
        },
    }
}

fn parse_duration_string(duration_string: &str) -> Result<Duration, WindowError> {
    let duration_string = duration_string
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    let duration_tokens = duration_string.split(' ').collect::<Vec<_>>();
    if duration_tokens.len() != 2 {
        return Err(WindowError::WindowInvalidInterval(duration_string));
    }

    let duration_value = duration_tokens[0]
        .parse::<i64>()
        .map_err(|_| WindowError::WindowInvalidInterval(duration_string.to_owned()))?;

    let duration_unit = duration_tokens[1].to_uppercase();

    match duration_unit.as_str() {
        "MILLISECOND" | "MILLISECONDS" => Ok(Duration::milliseconds(duration_value)),
        "SECOND" | "SECONDS" => Ok(Duration::seconds(duration_value)),
        "MINUTE" | "MINUTES" => Ok(Duration::minutes(duration_value)),
        "HOUR" | "HOURS" => Ok(Duration::hours(duration_value)),
        "DAY" | "DAYS" => Ok(Duration::days(duration_value)),
        _ => Err(WindowError::WindowInvalidInterval(duration_string)),
    }
}

pub fn string_from_sql_object_name(name: &ObjectName) -> String {
    name.0
        .iter()
        .map(ExpressionBuilder::normalize_ident)
        .collect::<Vec<String>>()
        .join(".")
}

pub fn get_field_index(ident: &[Ident], schema: &Schema) -> Result<Option<usize>, PipelineError> {
    let tables_matches = |table_ident: &Ident, fd: &FieldDefinition| -> bool {
        match fd.source.clone() {
            dozer_types::types::SourceDefinition::Table {
                connection: _,
                name,
            } => name == table_ident.value,
            dozer_types::types::SourceDefinition::Alias { name } => name == table_ident.value,
            dozer_types::types::SourceDefinition::Dynamic => false,
        }
    };

    let field_index = match ident.len() {
        1 => {
            let field_index = schema
                .fields
                .iter()
                .enumerate()
                .find(|(_, f)| f.name == ident[0].value)
                .map(|(idx, fd)| (idx, fd.clone()));
            field_index
        }
        2 => {
            let table_name = ident.first().expect("table_name is expected");
            let field_name = ident.last().expect("field_name is expected");

            let index = schema
                .fields
                .iter()
                .enumerate()
                .find(|(_, f)| tables_matches(table_name, f) && f.name == field_name.value)
                .map(|(idx, fd)| (idx, fd.clone()));
            index
        }
        // 3 => {
        //     let connection_name = comp_ident.get(0).expect("connection_name is expected");
        //     let table_name = comp_ident.get(1).expect("table_name is expected");
        //     let field_name = comp_ident.get(2).expect("field_name is expected");
        // }
        _ => {
            return Err(PipelineError::JoinError(JoinError::NameSpaceTooLong(
                ident
                    .iter()
                    .map(|a| a.value.clone())
                    .collect::<Vec<String>>()
                    .join("."),
            )));
        }
    };
    field_index.map_or(Ok(None), |(i, _fd)| Ok(Some(i)))
}
