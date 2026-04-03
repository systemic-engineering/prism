-module(prism_beam).
-compile([no_auto_import, nowarn_unused_vars, nowarn_unused_function, nowarn_nomatch, inline]).
-define(FILEPATH, "src/prism_beam.gleam").
-export([new/1, is_lossless/1, has_loss/1, was_recovered/1, map/2, with_step/2, with_loss/2, with_precision/2, with_recovery/2]).
-export_type([oid/0, shannon_loss/0, precision/0, pressure/0, recovery/0, beam/1]).

-if(?OTP_RELEASE >= 27).
-define(MODULEDOC(Str), -moduledoc(Str)).
-define(DOC(Str), -doc(Str)).
-else.
-define(MODULEDOC(Str), -compile([])).
-define(DOC(Str), -compile([])).
-endif.

?MODULEDOC(
    " Prism types for the BEAM.\n"
    " Mirrors the Rust prism crate. Type surface equivalence.\n"
).

-type oid() :: {oid, binary()}.

-type shannon_loss() :: {shannon_loss, float()}.

-type precision() :: {precision, float()}.

-type pressure() :: {pressure, float()}.

-type recovery() :: {coarsened, precision(), precision()} |
    {replayed, integer()} |
    {failed, binary()}.

-type beam(DQN) :: {beam,
        DQN,
        list(oid()),
        shannon_loss(),
        precision(),
        gleam@option:option(recovery())}.

-file("src/prism_beam.gleam", 47).
?DOC(" Create a beam with a result and no loss.\n").
-spec new(DQO) -> beam(DQO).
new(Result) ->
    {beam, Result, [], {shannon_loss, +0.0}, {precision, +0.0}, none}.

-file("src/prism_beam.gleam", 58).
?DOC(" Whether the projection was lossless.\n").
-spec is_lossless(beam(any())) -> boolean().
is_lossless(Beam) ->
    erlang:element(2, erlang:element(4, Beam)) =:= +0.0.

-file("src/prism_beam.gleam", 63).
?DOC(" Whether the projection lost information.\n").
-spec has_loss(beam(any())) -> boolean().
has_loss(Beam) ->
    erlang:element(2, erlang:element(4, Beam)) > +0.0.

-file("src/prism_beam.gleam", 68).
?DOC(" Whether recovery was attempted.\n").
-spec was_recovered(beam(any())) -> boolean().
was_recovered(Beam) ->
    gleam@option:is_some(erlang:element(6, Beam)).

-file("src/prism_beam.gleam", 73).
?DOC(" Map the result, preserving the trace.\n").
-spec map(beam(DQW), fun((DQW) -> DQY)) -> beam(DQY).
map(Beam, F) ->
    {beam,
        F(erlang:element(2, Beam)),
        erlang:element(3, Beam),
        erlang:element(4, Beam),
        erlang:element(5, Beam),
        erlang:element(6, Beam)}.

-file("src/prism_beam.gleam", 84).
?DOC(" Add a step to the path.\n").
-spec with_step(beam(DRA), oid()) -> beam(DRA).
with_step(Beam, Oid) ->
    {beam,
        erlang:element(2, Beam),
        lists:append(erlang:element(3, Beam), [Oid]),
        erlang:element(4, Beam),
        erlang:element(5, Beam),
        erlang:element(6, Beam)}.

-file("src/prism_beam.gleam", 89).
?DOC(" Set the loss.\n").
-spec with_loss(beam(DRD), shannon_loss()) -> beam(DRD).
with_loss(Beam, Loss) ->
    {beam,
        erlang:element(2, Beam),
        erlang:element(3, Beam),
        Loss,
        erlang:element(5, Beam),
        erlang:element(6, Beam)}.

-file("src/prism_beam.gleam", 94).
?DOC(" Set the precision.\n").
-spec with_precision(beam(DRG), precision()) -> beam(DRG).
with_precision(Beam, Precision) ->
    {beam,
        erlang:element(2, Beam),
        erlang:element(3, Beam),
        erlang:element(4, Beam),
        Precision,
        erlang:element(6, Beam)}.

-file("src/prism_beam.gleam", 99).
?DOC(" Set recovery.\n").
-spec with_recovery(beam(DRJ), recovery()) -> beam(DRJ).
with_recovery(Beam, Recovery) ->
    {beam,
        erlang:element(2, Beam),
        erlang:element(3, Beam),
        erlang:element(4, Beam),
        erlang:element(5, Beam),
        {some, Recovery}}.
