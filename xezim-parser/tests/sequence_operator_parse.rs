//! §16.9/§16.12 sequence and property operators that a bus-protocol assertion
//! library leans on, and that the parser used to reject outright:
//!
//!   * §16.9.7 `first_match(...)` — no parser arm at all, so the whole
//!     assertion failed with "expected expression, found KwFirst_match".
//!   * §16.12.7 `if (expr) property_expr [else property_expr]` — likewise
//!     absent: "expected expression, found KwIf".
//!   * §16.9.2 a cycle delay followed by a SAMPLED-VALUE FUNCTION
//!     (`##[0:16] $rose(x)`). `SystemIdentifier` was missing from the set of
//!     tokens that may START the delay's right operand, so the delay parsed as
//!     a bare unary and the `$rose` was left for the enclosing context. Inside
//!     parentheses that surfaced as the misleading "expected RParen, found
//!     $rose" — while the same shape WITHOUT a system function
//!     (`(##[0:16] x)`) parsed cleanly, which is what made it hard to spot.
//!
//! Signal and property names here are deliberately generic (`clk`, `req`,
//! `ack`, `busy`) and match no upstream design.

use sv_parser::parse;

fn errors(src: &str) -> Vec<String> {
    parse(src).errors.iter().map(|e| format!("{:?}", e)).collect()
}

/// Wrap a property expression in a minimal module so each case is one line.
fn prop(body: &str) -> String {
    format!(
        r#"
module m(input logic clk, rst, req, ack, busy);
  property p; @(posedge clk) disable iff (!rst) {body}; endproperty
  ap: assert property (p);
endmodule
"#
    )
}

// ── §16.9.7 first_match ────────────────────────────────────────────────

#[test]
fn first_match_parses_around_a_simple_sequence() {
    let e = errors(&prop("req |-> first_match(ack)"));
    assert!(e.is_empty(), "first_match must parse, got: {:?}", e);
}

#[test]
fn first_match_parses_around_a_delayed_sampled_value() {
    // The shape a "request eventually acknowledged" check takes.
    let e = errors(&prop("req |-> first_match((##[0:64] $fell(busy)))"));
    assert!(e.is_empty(), "first_match over a delay must parse, got: {:?}", e);
}

// ── §16.12.7 property if / if-else ─────────────────────────────────────

#[test]
fn property_if_without_else_parses() {
    let e = errors(&prop("if (req) ack"));
    assert!(e.is_empty(), "property `if` must parse, got: {:?}", e);
}

#[test]
fn property_if_else_parses() {
    let e = errors(&prop("if (req) ack else busy"));
    assert!(e.is_empty(), "property `if/else` must parse, got: {:?}", e);
}

#[test]
fn property_if_else_chains_and_takes_sequences() {
    let e = errors(&prop("if (req) ##2 ack else if (busy) ##1 ack"));
    assert!(e.is_empty(), "chained property `if` must parse, got: {:?}", e);
}

// ── §16.9.2 delay followed by a sampled-value function ─────────────────

#[test]
fn parenthesised_delay_then_sampled_value_parses() {
    let e = errors(&prop("req |-> (##[0:16] $rose(ack))"));
    assert!(
        e.is_empty(),
        "`(##[0:N] $rose(x))` must parse — the operand may start with a \
         system function, got: {:?}",
        e
    );
}

#[test]
fn fixed_delay_then_sampled_value_parses() {
    let e = errors(&prop("req |-> (##3 $rose(ack))"));
    assert!(e.is_empty(), "`(##N $rose(x))` must parse, got: {:?}", e);
}

#[test]
fn fixed_delay_then_unary_expression_parses() {
    let e = errors(&prop("req |-> (##1 !ack)"));
    assert!(e.is_empty(), "a delayed unary expression must parse, got: {:?}", e);
}

#[test]
fn stability_across_a_delayed_sampled_value_parses() {
    // `throughout` over a delayed sampled value: valid-and-stable until ready.
    let e = errors(&prop(
        "$rose(req) && (ack === 0) |=> ($stable(req) throughout (##[0:16] $rose(ack)))",
    ));
    assert!(e.is_empty(), "throughout over a delay must parse, got: {:?}", e);
}

// ── regression guards: shapes that already worked must keep working ────

#[test]
fn plain_delay_forms_still_parse() {
    for body in [
        "req ##[0:3] ack",
        "##[0:3] ack",
        "##3 ack",
        "req |-> (##[0:3] ack)",
        "req throughout ack",
        "req |-> ack",
        "$rose(req) |=> ack",
    ] {
        let e = errors(&prop(body));
        assert!(e.is_empty(), "`{}` must still parse, got: {:?}", body, e);
    }
}

/// `if` must still parse as a procedural STATEMENT — the property arm must not
/// steal it.
#[test]
fn procedural_if_still_parses() {
    let src = r#"
module m(input logic clk, req);
  logic q;
  always @(posedge clk) if (req) q <= 1; else q <= 0;
endmodule
"#;
    assert!(errors(src).is_empty());
}
