-module(prism_beam_test).
-compile([no_auto_import, nowarn_unused_vars, nowarn_unused_function, nowarn_nomatch, inline]).
-define(FILEPATH, "test/prism_beam_test.gleam").
-export([main/0, new_beam_is_lossless_test/0, beam_with_loss_test/0, beam_map_test/0, beam_with_step_test/0, beam_with_precision_test/0, beam_result_always_present_test/0, beam_not_recovered_by_default_test/0]).

-file("test/prism_beam_test.gleam", 5).
-spec main() -> nil.
main() ->
    gleeunit:main().

-file("test/prism_beam_test.gleam", 9).
-spec new_beam_is_lossless_test() -> nil.
new_beam_is_lossless_test() ->
    B = prism_beam:new(42),
    gleeunit@should:be_true(prism_beam:is_lossless(B)),
    gleeunit@should:be_false(prism_beam:has_loss(B)).

-file("test/prism_beam_test.gleam", 15).
-spec beam_with_loss_test() -> nil.
beam_with_loss_test() ->
    B = begin
        _pipe = prism_beam:new(0),
        prism_beam:with_loss(_pipe, {shannon_loss, 1.5})
    end,
    gleeunit@should:be_true(prism_beam:has_loss(B)),
    gleeunit@should:be_false(prism_beam:is_lossless(B)).

-file("test/prism_beam_test.gleam", 23).
-spec beam_map_test() -> nil.
beam_map_test() ->
    B = begin
        _pipe = prism_beam:new(10),
        prism_beam:map(_pipe, fun(X) -> X * 2 end)
    end,
    gleeunit@should:equal(erlang:element(2, B), 20).

-file("test/prism_beam_test.gleam", 30).
-spec beam_with_step_test() -> nil.
beam_with_step_test() ->
    B = begin
        _pipe = prism_beam:new(<<"data"/utf8>>),
        prism_beam:with_step(_pipe, {oid, <<"step1"/utf8>>})
    end,
    gleeunit@should:equal(erlang:element(3, B), [{oid, <<"step1"/utf8>>}]).

-file("test/prism_beam_test.gleam", 37).
-spec beam_with_precision_test() -> nil.
beam_with_precision_test() ->
    B = begin
        _pipe = prism_beam:new(1),
        prism_beam:with_precision(_pipe, {precision, 0.001})
    end,
    gleeunit@should:equal(erlang:element(5, B), {precision, 0.001}).

-file("test/prism_beam_test.gleam", 44).
-spec beam_result_always_present_test() -> nil.
beam_result_always_present_test() ->
    B = prism_beam:new(<<"always here"/utf8>>),
    gleeunit@should:equal(erlang:element(2, B), <<"always here"/utf8>>).

-file("test/prism_beam_test.gleam", 49).
-spec beam_not_recovered_by_default_test() -> nil.
beam_not_recovered_by_default_test() ->
    B = prism_beam:new(0),
    gleeunit@should:be_false(prism_beam:was_recovered(B)).
