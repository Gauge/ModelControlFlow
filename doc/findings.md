# Findings

| | |
|---|---|
| **Type** | Record — what a prototype or a run established, and what it changed |
| **Version** | 102 |
| **Status** | Living |
| **Authority** | Reports to [document-of-intent.md](document-of-intent.md) v25; a finding that changes intent is migrated there and cited from here |
| **Registers to** | [backlog.md](backlog.md) |

**A finding is a thing MCF learned by running something, written down with the
conditions it was learned under.** §8 requires that the reasoning behind a
change outlive the change; this file is where the *evidence* behind one lives,
so that an amendment to the intent document can cite something rather than
assert something.

Read it when a decision cites a run, or before proposing to reopen one.

**These are not measurements MCF publishes.** Every figure here was taken on
one machine, by a prototype, outside the exclusive window B35 requires for
timing-class work. They are evidence about *MCF*, adequate for deciding whether
a substrate or a budget is right, and inadequate for any claim about a model or
about hardware. A20's rule that an estimate is never promoted applies to them
in spirit: a prototype's reading is replaced by the suite's, never carried
forward as one.

## Contents

| § | Section |
|---|---|
| 1 | [F1 — The adversarial prototype (§7.19, DEC-019)](#1--f1--the-adversarial-prototype-719-dec-019) |
| 2 | [F2 — The development machine cannot attribute a budget (DEC-051)](#2--f2--the-development-machine-cannot-attribute-a-budget-dec-051) |
| 3 | [F3 — The load average answers the wrong question, and the obvious fix silently could not fail (DEC-051, D30)](#3--f3--the-load-average-answers-the-wrong-question-and-the-obvious-fix-silently-could-not-fail-dec-051-d30) |
| 4 | [F4 — What the new tiers found on their first runs (B-191)](#4--f4--what-the-new-tiers-found-on-their-first-runs-b-191) |
| 5 | [F5 — The cold-start budget is a measurement of the filesystem (B-011, D30)](#5--f5--the-cold-start-budget-is-a-measurement-of-the-filesystem-b-011-d30) |
| 6 | [F6 — The first mutant to survive (B-186)](#6--f6--the-first-mutant-to-survive-b-186) |
| 7 | [F7 — A major page fault is the signal F5 was missing (B-193)](#7--f7--a-major-page-fault-is-the-signal-f5-was-missing-b-193) |
| 8 | [F8 — How far a kernel MCF could maintain is from a specialist's (DEC-004)](#8--f8--how-far-a-kernel-mcf-could-maintain-is-from-a-specialists-dec-004) |
| 9 | [F9 — What a network costs, and what the hub actually does (B-021)](#9--f9--what-a-network-costs-and-what-the-hub-actually-does-b-021) |
| 10 | [F10 — What a machine says when a network is missing (DEC-011, D33)](#10--f10--what-a-machine-says-when-a-network-is-missing-dec-011-d33) |
| 11 | [F11 — A disk fills at the flush, not at the write (B-026)](#11--f11--a-disk-fills-at-the-flush-not-at-the-write-b-026) |
| 12 | [F12 — What an engine would cost, as far as it has been measured (B-320)](#12--f12--what-an-engine-would-cost-as-far-as-it-has-been-measured-b-320) |
| 13 | [F13 — Two writers, one record (DEC-037)](#13--f13--two-writers-one-record-dec-037) |
| 14 | [F14 — What a query over the record costs, and what an SQL engine would cost to ship (B-300, B-042, D6, D20)](#14--f14--what-a-query-over-the-record-costs-and-what-an-sql-engine-would-cost-to-ship-b-300-b-042-d6-d20) |
| 15 | [F15 — What actually needs elevation, asked of a machine (DEC-039, §7.39, §6.32)](#15--f15--what-actually-needs-elevation-asked-of-a-machine-dec-039-739-632) |
| 16 | [F16 — Three defects the real reference model found, and none of them was the one expected (B-213, B-019, §3.7)](#16--f16--three-defects-the-real-reference-model-found-and-none-of-them-was-the-one-expected-b-213-b-019-37) |
| 17 | [F17 — What a hub says when something is not there (DEC-038, §7.38)](#17--f17--what-a-hub-says-when-something-is-not-there-dec-038-738) |
| 18 | [F18 — The wall three findings kept hitting is a packaged toolchain (B-320, B-183, DEC-047)](#18--f18--the-wall-three-findings-kept-hitting-is-a-packaged-toolchain-b-320-b-183-dec-047) |
| 19 | [F19 — Two defects a real model found in an hour, and the tests that could not (B-364, B-365, A19, D38)](#19--f19--two-defects-a-real-model-found-in-an-hour-and-the-tests-that-could-not-b-364-b-365-a19-d38) |
| 20 | [F20 — A second architecture, and two silences that fail differently (B-365, A2, A19, D26)](#20--f20--a-second-architecture-and-two-silences-that-fail-differently-b-365-a2-a19-d26) |
| 21 | [F21 — Six families for a tenth of one model's bytes (B-369, D40, DEC-054, §3.12)](#21--f21--six-families-for-a-tenth-of-one-models-bytes-b-369-d40-dec-054-312) |
| 22 | [F22 — Where a model becomes able to referee an engine (B-370, D40, DEC-054, A19)](#22--f22--where-a-model-becomes-able-to-referee-an-engine-b-370-d40-dec-054-a19) |
| 23 | [F23 — Four expressions where MCF had two, and what no test could have found (B-365, B-370, A7, A19, A21)](#23--f23--four-expressions-where-mcf-had-two-and-what-no-test-could-have-found-b-365-b-370-a7-a19-a21) |
| 24 | [F24 — A third architecture, and three habits that fail three different ways (B-365, B-370, A19, §3.18)](#24--f24--a-third-architecture-and-three-habits-that-fail-three-different-ways-b-365-b-370-a19-318) |
| 25 | [F25 — The broken engine read better than the correct one (B-365, B-368, A19, D38)](#25--f25--the-broken-engine-read-better-than-the-correct-one-b-365-b-368-a19-d38) |
| 26 | [F26 — The oracle found a defect on its first run, and it was an invention of MCF's own (B-368, A19, A21, §3.12)](#26--f26--the-oracle-found-a-defect-on-its-first-run-and-it-was-an-invention-of-mcfs-own-b-368-a19-a21-312) |
| 27 | [F27 — What a coin-flip looks like, and what a defect looks like (B-368, B-365, A19, A21)](#27--f27--what-a-coin-flip-looks-like-and-what-a-defect-looks-like-b-368-b-365-a19-a21) |
| 28 | [F28 — The mask, past the boundary (B-368, F27, A21)](#28--f28--the-mask-past-the-boundary-b-368-f27-a21) |
| 29 | [F29 — The sixth family answers a different question (B-371, DEC-055, B-365, A19, §3.3)](#29--f29--the-sixth-family-answers-a-different-question-b-371-dec-055-b-365-a19-33) |
| 30 | [F30 — Two ways to provision the same component, measured (DEC-052, B-367, D39, A27)](#30--f30--two-ways-to-provision-the-same-component-measured-dec-052-b-367-d39-a27) |
| 31 | [F31 — What the first automated provisioning found in two tries (B-367, DEC-052, A27, §3.15)](#31--f31--what-the-first-automated-provisioning-found-in-two-tries-b-367-dec-052-a27-315) |
| 32 | [F32 — A defect the oracle let through, and the hole it came through (B-364, B-368, F27, A19, A21)](#32--f32--a-defect-the-oracle-let-through-and-the-hole-it-came-through-b-364-b-368-f27-a19-a21) |
| 33 | [F33 — The last schemes, and the widest noise (B-364, B-368, F32, A21)](#33--f33--the-last-schemes-and-the-widest-noise-b-364-b-368-f32-a21) |
| 34 | [F34 — Distributions against distributions (B-373, B-368, F27, F32, F33, A19)](#34--f34--distributions-against-distributions-b-373-b-368-f27-f32-f33-a19) |
| 35 | [F35 — What a load costs, apart from running (DEC-018, D41, B-034, §3.13)](#35--f35--what-a-load-costs-apart-from-running-dec-018-d41-b-034-313) |
| 36 | [F36 — The reference model answers, through an engine that is a process (B-032, B-033, B-367, D39, §XII)](#36--f36--the-reference-model-answers-through-an-engine-that-is-a-process-b-032-b-033-b-367-d39-xii) |
| 37 | [F37 — The first probe found two defects and then refused to answer (B-051, B-052, D42, §3.18, F25, F26)](#37--f37--the-first-probe-found-two-defects-and-then-refused-to-answer-b-051-b-052-d42-318-f25-f26) |
| 38 | [F38 — The probe's answer was upside down, and the decisive column was the broken one (B-052, B-374, D42, §3.18, F25, F37)](#38--f38--the-probes-answer-was-upside-down-and-the-decisive-column-was-the-broken-one-b-052-b-374-d42-318-f25-f37) |
| 39 | [F39 — The engine that can be probed, and three ways the instrument stood in for the model (B-032, B-055, B-376, D39, D42, F36, F38)](#39--f39--the-engine-that-can-be-probed-and-three-ways-the-instrument-stood-in-for-the-model-b-032-b-055-b-376-d39-d42-f36-f38) |
| 40 | [F40 — MCF's engine agrees with the reference for seven hundred positions (B-368, B-373, B-377, D39, A19, F27, F32, F39)](#40--f40--mcfs-engine-agrees-with-the-reference-for-seven-hundred-positions-and-the-rule-that-would-have-called-it-broken-was-the-wrong-rule-b-368-b-373-b-377-d39-a19-f27-f32-f39) |
| 41 | [F41 — The check could not fail, and the mutation that showed it was not the first one tried (B-003, B-368, B-377, A13, F40)](#41--f41--the-check-could-not-fail-and-the-mutation-that-showed-it-was-not-the-first-one-tried-b-003-b-368-b-377-a13-f40) |
| 42 | [F42 — The context both models declare is the context they have, and the probe that asked broke the protocol asking (B-055, B-058, B-059, B-376, D42, §3.7, §3.8, A2, A21)](#42--f42--the-context-both-models-declare-is-the-context-they-have-and-the-probe-that-asked-broke-the-protocol-asking-b-055-b-058-b-059-b-376-d42-37-38-a2-a21) |
| 43 | [F43 — A model that produced nothing now answers, and the change can be accounted for (B-059, B-062, D42, D43, §3.8, §3.15, A21, F38)](#43--f43--a-model-that-produced-nothing-now-answers-and-the-change-can-be-accounted-for-b-059-b-062-d42-d43-38-315-a21-f38) |
| 44 | [F44 — A configuration says whether it still holds, and the probe that asked spent eight thousand passes learning it could not (B-058, B-059, D42, D43, A21, F39, F42)](#44--f44--a-configuration-says-whether-it-still-holds-and-the-probe-that-asked-spent-eight-thousand-passes-learning-it-could-not-b-058-b-059-d42-d43-a21-f39-f42) |
| 45 | [F45 — The page whose job is to have no hidden choices had one, and the check for a moved condition invented one (B-058, B-059, B-062, D43, §3.15, A21, F44)](#45--f45--the-page-whose-job-is-to-have-no-hidden-choices-had-one-and-the-check-for-a-moved-condition-invented-one-b-058-b-059-b-062-d43-315-a21-f44) |
| 46 | [F46 — MCF's own default was cutting every answer off, and a gate test that depends on whether a daemon is running (B-056, B-059, D42, D43, §3.8, §3.12, B49, F38)](#46--f46--mcfs-own-default-was-cutting-every-answer-off-and-a-gate-test-that-depends-on-whether-a-daemon-is-running-b-056-b-059-d42-d43-38-312-b49-f38) |
| 47 | [F47 — The suite was reporting on the machine it found (B-378, B-003, B16, §3.12, F46)](#47--f47--the-suite-was-reporting-on-the-machine-it-found-b-378-b-003-b16-312-f46) |
| 48 | [F48 — The tie was MCF's, not the model's: a template that names a role in order to rename it (B-375, B-376, D42, D46, §3.7, F38, F39, F40)](#48--f48--the-tie-was-mcfs-not-the-models-a-template-that-names-a-role-in-order-to-rename-it-b-375-b-376-d42-d46-37-f38-f39-f40) |
| 49 | [F49 — MCF can check its own engine against the one it built, on a user's machine (B-362, B-376, D31, D39, A12, A19, §II)](#49--f49--mcf-can-check-its-own-engine-against-the-one-it-built-on-a-users-machine-b-362-b-376-d31-d39-a12-a19-ii) |
| 50 | [F50 — The privileged helper could be told where the machine is (B-190, D35, §6.32, §XVII, A2, A26)](#50--f50--the-privileged-helper-could-be-told-where-the-machine-is-b-190-d35-632-xvii-a2-a26) |
| 51 | [F51 — Contention moves the level, not the spread, and that decides how a benchmark must be built (DEC-007, B-250, B-181, §3.4, A19, F2, F3)](#51--f51--contention-moves-the-level-not-the-spread-and-that-decides-how-a-benchmark-must-be-built-dec-007-b-250-b-181-34-a19-f2-f3) |
| 52 | [F52 — The engine benchmarks will use is three times noisier than the one they will not (DEC-007, B-366, B-376, §3.4, A19, F51)](#52--f52--the-engine-benchmarks-will-use-is-three-times-noisier-than-the-one-they-will-not-and-two-guesses-about-why-were-both-wrong-dec-007-b-366-b-376-34-a19-f51) |
| 53 | [F53 — The noise floor is a property of the moment, not of the machine (DEC-007, B-083, B-181, D35, A19, F51, F52)](#53--f53--the-noise-floor-is-a-property-of-the-moment-not-of-the-machine-dec-007-b-083-b-181-d35-a19-f51-f52) |
| 54 | [F54 — The stopping condition, and the first comparison that stopped itself (B-083, B-086, B-250, DEC-007, F51, F52, F53)](#54--f54--the-stopping-condition-and-the-first-comparison-that-stopped-itself-b-083-b-086-b-250-dec-007-f51-f52-f53) |
| 55 | [F55 — The pairing is worth eighty-eight percent, and the stopping condition could only answer one way (B-250, B-083, B53, §3.27, DEC-007, F51, F53, F54)](#55--f55--the-pairing-is-worth-eighty-eight-percent-and-the-stopping-condition-could-only-answer-one-way-b-250-b-083-b53-327-dec-007-f51-f53-f54) |
| 56 | [F56 — A confounded comparison has no delta to give (A8, B-085, §3.4, A7, F55)](#56--f56--a-confounded-comparison-has-no-delta-to-give-a8-b-085-34-a7-f55) |
| 57 | [F57 — The sign test, and the resampling that could not see identical arms (B-086, B-083, DEC-007, A9, F51, F55)](#57--f57--the-sign-test-and-the-resampling-that-could-not-see-identical-arms-b-086-b-083-dec-007-a9-f51-f55) |
| 58 | [F58 — A null result reaches the disk as a result (A9, B-086, B-213, §6.3, D16, F56)](#58--f58--a-null-result-reaches-the-disk-as-a-result-a9-b-086-b-213-63-d16-f56) |
| 59 | [F59 — The benchmark runner, and the first real comparison it refused to over-report (B-080, A18, §6.7, B65, B-091, F53, F57)](#59--f59--the-benchmark-runner-and-the-first-real-comparison-it-refused-to-over-report-b-080-a18-67-b65-b-091-f53-f57) |
| 60 | [F60 — The clock in the type stopped a comparison, not a record (A11, B-082, D9, F59)](#60--f60--the-clock-in-the-type-stopped-a-comparison-not-a-record-a11-b-082-d9-f59) |
| 61 | [F61 — The seed set had to become arithmetic, and EINTR was being called a cut-off transfer (B-290, B61, D19, A2, F53, F55)](#61--f61--the-seed-set-had-to-become-arithmetic-and-eintr-was-being-called-a-cut-off-transfer-b-290-b61-d19-a2-f53-f55) |
| 62 | [F62 — The seed set is shown representative, and the sampler that would have cleared it for nothing (B-291, D19, §6.16, §7.13, B65)](#62--f62--the-seed-set-is-shown-representative-and-the-sampler-that-would-have-cleared-it-for-nothing-b-291-d19-616-713-b65) |
| 63 | [F63 — The recommendation is in a different repository from the weights (B-281, B60, D18, A21, §3.15)](#63--f63--the-recommendation-is-in-a-different-repository-from-the-weights-b-281-b60-d18-a21-315) |
| 64 | [F64 — Every benchmark trial was cold, and three fifths of it was process start (B-081, §6.13, B-376, F59, D41, F35)](#64--f64--every-benchmark-trial-was-cold-and-three-fifths-of-it-was-process-start-b-081-613-b-376-f59-d41-f35) |
| 65 | [F65 — One resident model and paired interleaving cannot both be had (B-090, B-081, B-250, DEC-001, §6.13, B53, F64)](#65--f65--one-resident-model-and-paired-interleaving-cannot-both-be-had-b-090-b-081-b-250-dec-001-613-b53-f64) |
| 66 | [F66 — Fifty-eight of a hundred trials is fifty-eight data points, and the fifty-ninth was not one (A4, B-087, §3.1, A1, A6)](#66--f66--fifty-eight-of-a-hundred-trials-is-fifty-eight-data-points-and-the-fifty-ninth-was-not-one-a4-b-087-31-a1-a6) |
| 67 | [F67 — The first frontier, and what it is mostly a frontier of (B-091, §XII, §3.4, §3.27, F64, F65)](#67--f67--the-first-frontier-and-what-it-is-mostly-a-frontier-of-b-091-xii-34-327-f64-f65) |
| 68 | [F68 — The bundle's header said it held no user content while carrying the prompt (B-211, PR2, A24, A25, §II)](#68--f68--the-bundles-header-said-it-held-no-user-content-while-carrying-the-prompt-b-211-pr2-a24-a25-ii) |
| 69 | [F69 — A band predicted before the measurement, and the measurement landed in it (B-214, B-215, PR3, A20, B46, §6.16, F67)](#69--f69--a-band-predicted-before-the-measurement-and-the-measurement-landed-in-it-b-214-b-215-pr3-a20-b46-616-f67) |
| 70 | [F70 — A run that cannot decide names what it competed with, and I could not make one (B-216, PR5, §3.8, B24, B4, D25, F55)](#70--f70--a-run-that-cannot-decide-names-what-it-competed-with-and-i-could-not-make-one-b-216-pr5-38-b24-b4-d25-f55) |
| 71 | [F71 — The machine either side of a run is a condition, not a gate (B-217, DEC-007, §3.4, §3.8, A6, A7)](#71--f71--the-machine-either-side-of-a-run-is-a-condition-not-a-gate-b-217-dec-007-34-38-a6-a7) |
| 72 | [F72 — Work is counted; minutes are derived, banded, and sometimes absent (B-224, B-225, B46, D14, A20, A7)](#72--f72--work-is-counted-minutes-are-derived-banded-and-sometimes-absent-b-224-b-225-b46-d14-a20-a7) |
| 73 | [F73 — A budget proposes, and what it excluded is on the page (B-226, B47, §3.1, A7)](#73--f73--a-budget-proposes-and-what-it-excluded-is-on-the-page-b-226-b47-31-a7) |
| 74 | [F74 — The record caught a third party's workload, and the projection swallowed it (B-217, F71, §3.8, A6, B34)](#74--f74--the-record-caught-a-third-partys-workload-and-the-projection-swallowed-it-b-217-f71-38-a6-b34) |
| 75 | [F75 — A band that says what it rested on turns a wrong-looking number into a legible one (B-385, F74, §3.4, A6, A7)](#75--f75--a-band-that-says-what-it-rested-on-turns-a-wrong-looking-number-into-a-legible-one-b-385-f74-34-a6-a7) |
| 76 | [F76 — A run that reports as it goes shows what moves, not what has not decided (B-227, A4, §3.1, A18)](#76--f76--a-run-that-reports-as-it-goes-shows-what-moves-not-what-has-not-decided-b-227-a4-31-a18) |
| 77 | [F77 — Four outcomes and no total, built before the laboratories that will produce them (B-200, B-201, B40, B41, D2, §3.23, §3.9)](#77--f77--four-outcomes-and-no-total-built-before-the-laboratories-that-will-produce-them-b-200-b-201-b40-b41-d2-323-39) |
| 78 | [F78 — Eight Japanese characters cost fifteen tokens here and four there, and the shattering is visible (B-381, PR11, §3.15, F19, A1)](#78--f78--eight-japanese-characters-cost-fifteen-tokens-here-and-four-there-and-the-shattering-is-visible-b-381-pr11-315-f19-a1) |
| 79 | [F79 — A marker typed into a prompt is shown as what it becomes (B-383, PR11, F37, F26, D46, §3.7)](#79--f79--a-marker-typed-into-a-prompt-is-shown-as-what-it-becomes-b-383-pr11-f37-f26-d46-37) |
| 80 | [F80 — The register's headline was wrong by five items, and the check was green because it skipped what it could not parse (B-383, C5, A1, A2, §3.5)](#80--f80--the-registers-headline-was-wrong-by-five-items-and-the-check-was-green-because-it-skipped-what-it-could-not-parse-b-383-c5-a1-a2-35) |
| 81 | [F81 — Korean costs 6.5 times English on one vocabulary and 4.1 on another, and neither is a fact about Korean (B-379, §3.15, DEC-002)](#81--f81--korean-costs-65-times-english-on-one-vocabulary-and-41-on-another-and-neither-is-a-fact-about-korean-b-379-315-dec-002) |
| 82 | [F82 — A prompt's cost can be stated before it is sent, and the measured context is on a terminal and nowhere else (B-382, B-386, A21, A1, F42)](#82--f82--a-prompts-cost-can-be-stated-before-it-is-sent-and-the-measured-context-is-on-a-terminal-and-nowhere-else-b-382-b-386-a21-a1-f42) |
| 83 | [F83 — The probe writes it down, and the prompt is measured against what the machine takes (B-386, B-382, A1, A9, D42, F42)](#83--f83--the-probe-writes-it-down-and-the-prompt-is-measured-against-what-the-machine-takes-b-386-b-382-a1-a9-d42-f42) |
| 84 | [F84 — The branch is read from the attribution, and reading it from the category is the obvious wrong design (B-233, B24, §7.10, §3.4)](#84--f84--the-branch-is-read-from-the-attribution-and-reading-it-from-the-category-is-the-obvious-wrong-design-b-233-b24-710-34) |
| 85 | [F85 — An idle daemon took zero processor ticks and issued zero reads in ninety seconds (B-187, B-108, B4, D5, §3.13, §6.18)](#85--f85--an-idle-daemon-took-zero-processor-ticks-and-issued-zero-reads-in-ninety-seconds-b-187-b-108-b4-d5-313-618) |
| 86 | [F86 — A field of one is refused by name, and a foreign number has no route in (B-167, B-127, B34, B43, §6.23, §5)](#86--f86--a-field-of-one-is-refused-by-name-and-a-foreign-number-has-no-route-in-b-167-b-127-b34-b43-623-5) |
| 87 | [F87 — A contribution has nowhere to put a task, and no way to be unsent (B-171, B-203, B-251, B-310, B42, B54, D21, §6.30, §3.20)](#87--f87--a-contribution-has-nowhere-to-put-a-task-and-no-way-to-be-unsent-b-171-b-203-b-251-b-310-b42-b54-d21-630-320) |
| 88 | [F88 — A behaviour laboratory's bound has nowhere to put a wall clock (B-230, B-223, B45, D8, D13, §3.8)](#88--f88--a-behaviour-laboratorys-bound-has-nowhere-to-put-a-wall-clock-b-230-b-223-b45-d8-d13-38) |
| 89 | [F89 — A figure with a unit and nothing behind it is the most convincing kind of wrong (B-188, B-163, B-164, B39, B31, A20, A7)](#89--f89--a-figure-with-a-unit-and-nothing-behind-it-is-the-most-convincing-kind-of-wrong-b-188-b-163-b-164-b39-b31-a20-a7) |
| 90 | [F90 — The contention instrument reported 35 cores on a 32-thread machine, because it divided by the window it meant to use (B-216, B-217, DEC-007, A2, §3.8)](#90--f90--the-contention-instrument-reported-35-cores-on-a-32-thread-machine-because-it-divided-by-the-window-it-meant-to-use-b-216-b-217-dec-007-a2-38) |
| 91 | [F91 — The sensors were there the whole time, one directory across (B-084, DEC-007, A7, A2, §3.4)](#91--f91--the-sensors-were-there-the-whole-time-one-directory-across-b-084-dec-007-a7-a2-34) |
| 92 | [F92 — The headline number had no measure of itself, and the sentence beside it claimed otherwise (B46, B54, A6, §6.16, §3.27)](#92--f92--the-headline-number-had-no-measure-of-itself-and-the-sentence-beside-it-claimed-otherwise-b46-b54-a6-616-327) |
| 93 | [F93 — Every measurement MCF has taken is attributed to an instrument it cannot identify (§3.4, A1, A2, A7, §6.16)](#93--f93--every-measurement-mcf-has-taken-is-attributed-to-an-instrument-it-cannot-identify-34-a1-a2-a7-616) |
| 94 | [F94 — A19 was applied to everything MCF computes and nothing MCF measures (A19, §6.16, F90, F91, F92, F93)](#94--f94--a19-was-applied-to-everything-mcf-computes-and-nothing-mcf-measures-a19-616-f90-f91-f92-f93) |
| 95 | [F95 — The band is at a third of the machine, and half a machine free is not enough (DEC-007, B-217, B-084, §3.8, F90, F92)](#95--f95--the-band-is-at-a-third-of-the-machine-and-half-a-machine-free-is-not-enough-dec-007-b-217-b-084-38-f90-f92) |
| 96 | [F96 — The pre-flight marks rather than refuses, and the band it uses says whose machine measured it (B-217, DEC-007, F95, A21, A20, A4)](#96--f96--the-pre-flight-marks-rather-than-refuses-and-the-band-it-uses-says-whose-machine-measured-it-b-217-dec-007-f95-a21-a20-a4) |
| 97 | [F97 — Five modules measure something and are checked against nothing, and now they say so (B-390, A19, A7, §6.16)](#97--f97--five-modules-measure-something-and-are-checked-against-nothing-and-now-they-say-so-b-390-a19-a7-616) |
| 98 | [F98 — The unpaired interval needed different mathematics, and the recurrence was checked against enumeration (B-388, B54, B53, §3.27, A19)](#98--f98--the-unpaired-interval-needed-different-mathematics-and-the-recurrence-was-checked-against-enumeration-b-388-b54-b53-327-a19) |
| 99 | [F99 — Threads pay at every shape a model performs, cost nothing in noise, and past a product's optimum more of them is slower (B-366, D38, F52, §3.12, A6, A19)](#99--f99--threads-pay-at-every-shape-a-model-performs-cost-nothing-in-noise-and-past-a-products-optimum-more-of-them-is-slower-b-366-d38-f52-312-a6-a19) |
| 100 | [F100 — How large a model MCF's own engine can usefully read, and what actually stops it (B-384, PR12, D40, B-366, F99, A20, A7)](#100--f100--how-large-a-model-mcfs-own-engine-can-usefully-read-and-what-actually-stops-it-b-384-pr12-d40-b-366-f99-a20-a7) |
| 101 | [F101 — A model called the tool perfectly and the probe recorded *no call*, five times out of five (B-053, D42, A1, A2, A21, §3.18)](#101--f101--a-model-called-the-tool-perfectly-and-the-probe-recorded-no-call-five-times-out-of-five-b-053-d42-a1-a2-a21-318) |
| 102 | [F102 — The conformance corpus answered differently depending on whether a daemon was running (B-370, B-053, F46, F27, D40, §3.12, A19)](#102--f102--the-conformance-corpus-answered-differently-depending-on-whether-a-daemon-was-running-b-370-b-053-f46-f27-d40-312-a19) |
| 103 | [F103 — The oracle could compare the reference with itself, and a section that compared nothing read like one that passed (B-368, B-370, F102, F47, A19, A4, §3.12)](#103--f103--the-oracle-could-compare-the-reference-with-itself-and-a-section-that-compared-nothing-read-like-one-that-passed-b-368-b-370-f102-f47-a19-a4-312) |
| 104 | [F104 — The tier named the engine and still asked another binary, and the account could not tell (B-391, F103, F102, F93, §3.12, A19, A6)](#104--f104--the-tier-named-the-engine-and-still-asked-another-binary-and-the-account-could-not-tell-b-391-f103-f102-f93-312-a19-a6) |
| 105 | [F105 — A25's guarantee was structural and unused: 4 047 record entries held content, and the export said they did not (B-392, A25, A1, A24, §6.8, F68, F104, F103)](#105--f105--a25s-guarantee-was-structural-and-unused-4-047-record-entries-held-content-and-the-export-said-they-did-not-b-392-a25-a1-a24-68-f68-f104-f103) |
| 106 | [F106 — A probe that asks for a shape, and the four whose results were never written down (B-054, B-386, D42, A1, A7, A9, F101, F103, F105)](#106--f106--a-probe-that-asks-for-a-shape-and-the-four-whose-results-were-never-written-down-b-054-b-386-d42-a1-a7-a9-f101-f103-f105) |
| 107 | [F107 — The oracle's first disagreement in three days was the instrument's, not the engine's (B-393, B-368, B-373, F27, F34, F103, A19, A5)](#107--f107--the-oracles-first-disagreement-in-three-days-was-the-instruments-not-the-engines-b-393-b-368-b-373-f27-f34-f103-a19-a5) |
| 108 | [F108 — Two rules rested on somebody remembering, and one identifier had been cited four times with nothing behind it (B-394, C5, C6, B16, F80, §7.30)](#108--f108--two-rules-rested-on-somebody-remembering-and-one-identifier-had-been-cited-four-times-with-nothing-behind-it-b-394-c5-c6-b16-f80-730) |
| 109 | [F109 — A rule of thumb that cannot be read as a result, cannot be recorded, and expires when the laboratory lands (B-380, DEC-002, A21, A25, §3.15, §3.18)](#109--f109--a-rule-of-thumb-that-cannot-be-read-as-a-result-cannot-be-recorded-and-expires-when-the-laboratory-lands-b-380-dec-002-a21-a25-315-318) |
| 110 | [F110 — A probe for each modality, or a reason: what a language costs, whether an artifact embeds, and three declinations in writing (B-057, D42, F81, F106, A7, A21, §X)](#110--f110--a-probe-for-each-modality-or-a-reason-what-a-language-costs-whether-an-artifact-embeds-and-three-declinations-in-writing-b-057-d42-f81-f106-a7-a21-x) |
| 111 | [F111 — The budget tier fired on its first run in two days, and most of what it caught had been there for one of them (B-011, B20, D24, B-185, B38)](#111--f111--the-budget-tier-fired-on-its-first-run-in-two-days-and-most-of-what-it-caught-had-been-there-for-one-of-them-b-011-b20-d24-b-185-b38) |
| 112 | [F112 — Two absolute rules named checks that did not exist, and one of them had a hole on a shipped surface (B-073, B-072, A6, A22, B16, D27)](#112--f112--two-absolute-rules-named-checks-that-did-not-exist-and-one-of-them-had-a-hole-on-a-shipped-surface-b-073-b-072-a6-a22-b16-d27) |
| 113 | [F113 — The load tier found a race in the laboratory, and it was six runs in a thousand of a file being written and executed at once (B-191, B-009, §3.17, D26, A13)](#113--f113--the-load-tier-found-a-race-in-the-laboratory-and-it-was-six-runs-in-a-thousand-of-a-file-being-written-and-executed-at-once-b-191-b-009-317-d26-a13) |
| 114 | [F114 — MCF was keeping its own probe traffic in the store meant for the operator's private text (B-146, B-392, §6.8, A17, A25, B9)](#114--f114--mcf-was-keeping-its-own-probe-traffic-in-the-store-meant-for-the-operators-private-text-b-146-b-392-68-a17-a25-b9) |
| 115 | [F115 — A16's fifth gate was in the rule and nowhere in the code, and every comparison MCF had recorded was uncontributable for saying nothing (B-160, B-039, A16, A24, B-203, B42, §3.20)](#115--f115--a16s-fifth-gate-was-in-the-rule-and-nowhere-in-the-code-and-every-comparison-mcf-had-recorded-was-uncontributable-for-saying-nothing-b-160-b-039-a16-a24-b-203-b42-320) |
| 116 | [F116 — The benchmark's standard question, chosen by measuring 27 vocabularies rather than by taste (B-160, B42, F110, F81, §6.37)](#116--f116--the-benchmarks-standard-question-chosen-by-measuring-27-vocabularies-rather-than-by-taste-b-160-b42-f110-f81-637) |
| 117 | [F117 — The pinned length was declared and never enforced, and a rate computed the obvious way is a property of the length you chose (B-081, D19, A19, §3.4, F116)](#117--f117--the-pinned-length-was-declared-and-never-enforced-and-a-rate-computed-the-obvious-way-is-a-property-of-the-length-you-chose-b-081-d19-a19-34-f116) |
| 118 | [F118 — Generation is not a constant-rate process: the rate halves with context depth, and the depth a machine can reach is bounded by memory rather than by the model (B-396, B-397, D19, §3.4, A19, F117)](#118--f118--generation-is-not-a-constant-rate-process-the-rate-halves-with-context-depth-and-the-depth-a-machine-can-reach-is-bounded-by-memory-rather-than-by-the-model-b-396-b-397-d19-34-a19-f117) |
| 119 | [F119 — The allocation effect was the running order, and the fall-off is the memory bus (B-400, F118, A12, A21, D19)](#119-f119-the-allocation-effect-was-the-running-order-and-the-fall-off-is-the-memory-bus-b-400-f118-a12-a21-d19) |
| 120 | [F120 — A prefilled depth costs what a generated one costs and arrives twenty times sooner, which retires the long path rather than speeding it up (B-400, F119, A18, A11)](#120-f120-a-prefilled-depth-costs-what-a-generated-one-costs-and-arrives-twenty-times-sooner-which-retires-the-long-path-rather-than-speeding-it-up-b-400-f119-a18-a11) |
| 121 | [F121 — The fall-off predicted from the file header on all 27 models, and the one outlier was an architecture the arithmetic did not describe (B-400, F120, A7, A21, B16)](#121-f121-the-fall-off-predicted-from-the-file-header-on-all-27-models-and-the-one-outlier-was-an-architecture-the-arithmetic-did-not-describe-b-400-f120-a7-a21-b16) |
| 122 | [F122 — The predictor generalised by being made to refuse: two architectures agree to 8%, and every model it cannot describe now says so (B-400, F121, A2, A7, A9, A21)](#122-f122-the-predictor-generalised-by-being-made-to-refuse-two-architectures-agree-to-8-and-every-model-it-cannot-describe-now-says-so-b-400-f121-a2-a7-a9-a21) |
| 123 | [F123 — Counted rather than inferred: a token reads the whole model and the whole cache, so both halves of the curve are in the file (B-400, F121, F122, A11, A21, D19)](#123-f123-counted-rather-than-inferred-a-token-reads-the-whole-model-and-the-whole-cache-so-both-halves-of-the-curve-are-in-the-file-b-400-f121-f122-a11-a21-d19) |
| 124 | [F124 — The whole curve from the file, on seven architectures, with a state-space model as the control that has no curve at all (B-400, F123, A6, A7, A12, A21)](#124-f124-the-whole-curve-from-the-file-on-seven-architectures-with-a-state-space-model-as-the-control-that-has-no-curve-at-all-b-400-f123-a6-a7-a12-a21) |
| 125 | [F125 — Five ways to state a speed, scored against each other: one probe and the header beats measuring everything, and beats the file alone (B-400, F124, A6, A18, A20)](#125-f125-five-ways-to-state-a-speed-scored-against-each-other-one-probe-and-the-header-beats-measuring-everything-and-beats-the-file-alone-b-400-f124-a6-a18-a20) |
| 126 | [F126 — A probe's length is not a free knob: short probes read optimistically, and with that fixed every size to 70B fits five minutes (B-400, F125, A6, A20, D19)](#126-f126-a-probes-length-is-not-a-free-knob-short-probes-read-optimistically-and-with-that-fixed-every-size-to-70b-fits-five-minutes-b-400-f125-a6-a20-d19) |
| 127 | [F127 — The sampling rules made machine-checkable, and the GPU that cannot be tested because the engine has no backend for it (B-400, F126, A2, A21, B16)](#127-f127-the-sampling-rules-made-machine-checkable-and-the-gpu-that-cannot-be-tested-because-the-engine-has-no-backend-for-it-b-400-f126-a2-a21-b16) |
| 128 | [F128 — A CUDA build provisioned, the first GPU timings taken, and the CPU's constants do not transfer to it (B-400, F127, A12, A21, F31)](#128-f128-a-cuda-build-provisioned-the-first-gpu-timings-taken-and-the-cpus-constants-do-not-transfer-to-it-b-400-f127-a12-a21-f31) |
| 129 | [F129 — A second surface arrived and the tripwire watching for one did not fire, because it was written against four guesses at its name (B-401, B-072, A22, B16)](#129-f129-a-second-surface-arrived-and-the-tripwire-watching-for-one-did-not-fire-because-it-was-written-against-four-guesses-at-its-name-b-401-b-072-a22-b16) |
| 130 | [F130 — Two engines were provisioned and invisible, because discovery matched a name; and sixty-four sentences a person reads cite a document they have never seen (B-402, B-403, A21, A7, B16)](#130-f130-two-engines-were-provisioned-and-invisible-because-discovery-matched-a-name-and-sixty-four-sentences-a-person-reads-cite-a-document-they-have-never-seen-b-402-b-403-a21-a7-b16) |
| 131 | [F131 — The window was a terminal with a mouse pointer over it, and every test passed; one SDL constant was written from memory and named the wrong event (B-409, F129, A6, A11)](#131-f131-the-window-was-a-terminal-with-a-mouse-pointer-over-it-and-every-test-passed-one-sdl-constant-was-written-from-memory-and-named-the-wrong-event-b-409-f129-a6-a11) |
| 132 | [F132 — Three of the window's new capabilities were caught by checks before they shipped: a progress bar that reported zero at the moment it finished, a plan that sampled hardware from the serving path, and a timing in floating point (B-412, A7, B4, A6)](#132-f132-three-of-the-windows-new-capabilities-were-caught-by-checks-before-they-shipped-a-progress-bar-that-reported-zero-at-the-moment-it-finished-a-plan-that-sampled-hardware-from-the-serving-path-and-a-timing-in-floating-point-b-412-a7-b4-a6) |
| 133 | [F133 — MCF said a model ran on the graphics card and ran it on the processor: the layer count was written into the source as zero, and it cost 4.9× (B-416, A6, A12, §3.15)](#133-f133-mcf-said-a-model-ran-on-the-graphics-card-and-ran-it-on-the-processor-the-layer-count-was-written-into-the-source-as-zero-and-it-cost-49-b-416-a6-a12-3-15) |
| 134 | [F134 — A type MCF already had, written a second time: sampling in thousandths, without the one distinction the original carries (B-419, A1, B-281)](#134-f134-a-type-mcf-already-had-written-a-second-time-sampling-in-thousandths-without-the-one-distinction-the-original-carries-b-419-a1-b-281) |
| 153 | [F153 — The console's buttons could not be reached: Tab sits below the printable range, the arm that named it was dead, and the tests handed the screen a key the decoder never produced (B-404, B-401, A22, F130, F131)](#153-f153-the-consoles-buttons-could-not-be-reached-tab-sits-below-the-printable-range-the-arm-that-named-it-was-dead-and-the-tests-handed-the-screen-a-key-the-decoder-never-produced-b-404-b-401-a22-f130-f131) |
| 152 | [F152 — No rung deeper than 2,048 was ever measured: the served engine was reused by model alone, refused every turn longer than its first window, and the refusal was written down as a pair that did not separate (B-424, A2, A7, A9, F133)](#152-f152-no-rung-deeper-than-2048-was-ever-measured-the-served-engine-was-reused-by-model-alone-refused-every-turn-longer-than-its-first-window-and-the-refusal-was-written-down-as-a-pair-that-did-not-separate-b-424-a2-a7-a9-f133) |
| 151 | [F151 — A latent cache was withheld by the report and sized at nearly twice by placement, from one header read two ways; now one reading serves both (B-038, B-072, A7, F150)](#151-f151-a-latent-cache-was-withheld-by-the-report-and-sized-at-nearly-twice-by-placement-from-one-header-read-two-ways-now-one-reading-serves-both-b-038-b-072-a7-f150) |
| 150 | [F150 — The cache of a hybrid was sized by the header's block count and came out four times too large; the same file's heads read as thirty-two against sixteen declared, and its BF16 tensors as a type MCF does not read (B-038, A7, A21, F16)](#150-f150-the-cache-of-a-hybrid-was-sized-by-the-headers-block-count-and-came-out-four-times-too-large-the-same-files-heads-read-as-thirty-two-against-sixteen-declared-and-its-bf16-tensors-as-a-type-mcf-does-not-read-b-038-a7-a21-f16) |
| 149 | [F149 — The daemon found its engines once and never looked again, so an engine built while it ran was a prefix on disk and *no engine* on the socket (F31, A7, B-367, B-072)](#149-f149-the-daemon-found-its-engines-once-and-never-looked-again-so-an-engine-built-while-it-ran-was-a-prefix-on-disk-and-no-engine-on-the-socket-f31-a7-b-367-b-072) |
| 148 | [F148 — Six readers told the operator MCF did not say why, over a body that said exactly why (A2, B-072)](#148-f148-six-readers-told-the-operator-mcf-did-not-say-why-over-a-body-that-said-exactly-why-a2-b-072) |
| 147 | [F147 — One real prompt found five defects in prompt analysis, and the report presented a run that separated nothing exactly as it presents one that works (§3.15, A6, A7, §3.4)](#147-f147-one-real-prompt-found-five-defects-in-prompt-analysis-and-the-report-presented-a-run-that-separated-nothing-exactly-as-it-presents-one-that-works-3-15-a6-a7-3-4) |
| 146 | [F146 — The completion tool was never given a window, so every generation opened the model's whole trained context: 81.3 GB resident to produce a few hundred tokens (F133, A6, A12, §3.15)](#146-f146-the-completion-tool-was-never-given-a-window-so-every-generation-opened-the-models-whole-trained-context-813-gb-resident-to-produce-a-few-hundred-tokens-f133-a6-a12-3-15) |
| 145 | [F145 — `mcf probe` held the whole model in the console beside the engine already holding it: 24.7 GB for a 14.5 GB file, and the memory nothing could account for (B-372, A22, F140)](#145-f145-mcf-probe-held-the-whole-model-in-the-console-beside-the-engine-already-holding-it-247-gb-for-a-145-gb-file-and-the-memory-nothing-could-account-for-b-372-a22-f140) |
| 144 | [F144 — MCF planned against the machine while running inside a limit, and its fixed overhead constant is thirteen times too small (A21, A19, B-372)](#144-f144-mcf-planned-against-the-machine-while-running-inside-a-limit-and-its-fixed-overhead-constant-is-thirteen-times-too-small-a21-a19-b-372) |
| 143 | [F143 — Two surfaces counted one store and got ten and eleven: the daemon sent the distinction and the console dropped it (B-422, B-072)](#143-f143-two-surfaces-counted-one-store-and-got-ten-and-eleven-the-daemon-sent-the-distinction-and-the-console-dropped-it-b-422-b-072) |
| 142 | [F142 — The engine's own `[end of text]` was recorded as the model's words, and MCF reported it did not know why generation stopped while holding the thing that said (A19, A21, A2, A7)](#142-f142-the-engines-own-end-of-text-was-recorded-as-the-models-words-and-mcf-reported-it-did-not-know-why-generation-stopped-while-holding-the-thing-that-said-a19-a21-a2-a7) |
| 140 | [F140 — `mcf explain` read the whole model into memory to look at its header: 16.41 GB to describe a file, beneath a note in the same module saying it must not (§3.11, A2, B-372)](#140-f140-mcf-explain-read-the-whole-model-into-memory-to-look-at-its-header-1641-gb-to-describe-a-file-beneath-a-note-in-the-same-module-saying-it-must-not-3-11-a2-b-372) |
| 139 | [F139 — Eight commands MCF answers were absent from its own help, and six of them denied existing when typed: the console sent operators to two of them by name (A22, §3.15, A2, F135)](#139-f139-eight-commands-mcf-answers-were-absent-from-its-own-help-and-six-of-them-denied-existing-when-typed-the-console-sent-operators-to-two-of-them-by-name-a22-3-15-a2-f135) |
| 138 | [F138 — Four places sized a model and one of them counted the whole of it: a 111 GB model planned against as 10.9 MB, and a context recommended that would take the machine down (B-422, B-072, A21, F136)](#138-f138-four-places-sized-a-model-and-one-of-them-counted-the-whole-of-it-a-111-gb-model-planned-against-as-109-mb-and-a-context-recommended-that-would-take-the-machine-down-b-422-b-072-a21-f136) |
| 137 | [F137 — A supervisor that killed the one process it could name, then waited on the ones it could not: the deadline fired at 200 ms and the number beside it said thirty seconds (A3, A27, A4, B37)](#137-f137-a-supervisor-that-killed-the-one-process-it-could-name-then-waited-on-the-ones-it-could-not-the-deadline-fired-at-200-ms-and-the-number-beside-it-said-thirty-seconds-a3-a27-a4-b37) |
| 136 | [F136 — The console weighed a model before loading it and the daemon did not, so a machine with no room for one was told by the kernel, and not necessarily in this process (B-372, A2, A22, B-072)](#136-f136-the-console-weighed-a-model-before-loading-it-and-the-daemon-did-not-so-a-machine-with-no-room-for-one-was-told-by-the-kernel-and-not-necessarily-in-this-process-b-372-a2-a22-b-072) |
| 135 | [F135 — One change made three register entries false, and none of them said so: a status is prose, and prose does not fail a build (B-036, B-038, B-040, A1)](#135-f135-one-change-made-three-register-entries-false-and-none-of-them-said-so-a-status-is-prose-and-prose-does-not-fail-a-build-b-036-b-038-b-040-a1) |
| — | [Changelog](#changelog) |

## 1 · F1 — The adversarial prototype (§7.19, DEC-019)

**What was run.** `prototypes/adversarial` (B-002): interrogate an accelerator
by two routes, supervise four child processes deliberately made to die badly,
classify everything against [taxonomy.md](taxonomy.md), and measure MCF's own
cost against D24.

**Conditions.** One machine: a 16-core AMD Ryzen 9 9950X, 91 GiB of memory, one
NVIDIA GeForce RTX 5080 on driver 610.57.04, Linux 7.1.9, `x86_64-unknown-linux-gnu`.
Rust 1.98.0, release profile. The machine was **not** quiet — it was building
this repository at the same time for one of the runs below, and that run is
reported precisely because the difference is the finding.

**Verdict: D4 is confirmed. §7.19 is closed by DEC-019 without amendment.**

### 1.1 The supervision claim holds, and costs little

Four scenarios, four distinct taxonomy categories, all reached without the
supervisor being affected:

| The child | Category reached | Disposition | Partial output |
|---|---|---|---|
| Is not there to spawn | `engine.spawn.not_found` | `refused` | none |
| Exits 3 before any output | `engine.exit.immediate` | `aborted` | none |
| Prints 18 bytes, then `kill -9 $$` | `engine.exit.signal` | `partial` | **18 bytes kept** |
| Prints 7 bytes, then exits 1 | `engine.exit.midstream` | `partial` | **7 bytes kept** |
| Sleeps 30 s, silent | `engine.hang.no_output` | `aborted` | none |

A3's claim — *the manager survives the managed* — is the one that had to be
demonstrated rather than argued, and it is: the supervisor returns from every
one of these, including the signal death and the hang. A4's claim holds
alongside it: the bytes a child emitted before dying are kept and the
disposition says `partial`, rather than being discarded with the child.

**What this cost in code.** The supervisor is about 180 lines with no
dependency, and the only subtlety is one D4 predicts: the child's output has to
be drained on another thread, or a silent child makes the deadline
unenforceable. That is a concurrency hazard, and it is the kind Rust makes hard
to get wrong by accident.

### 1.2 The C-ABI claim holds, and it matters more than expected

D4's third argument is that accelerator interrogation is "constant C-ABI work,
and Rust pays no tax at that boundary". The prototype probes the same device
twice to test it:

| Route | Answered | What it could not say |
|---|---|---|
| Files the driver publishes (`/proc`, no `unsafe`) | 3 of 5 | device memory, temperature |
| The vendor management library, over the C ABI | **5 of 5** | — |

The file route names the vendor, the model and the driver version and stops.
The vendor library adds the two that are *live state* — 17 094 934 528 bytes of
device memory and 27 °C — and those are precisely the two §3.4 and §3.8 need,
because they vary and they change a result.

**The finding is not that FFI works. It is that the file route is not
sufficient.** A profiler built only on published files would report a device's
identity and be unable to say whether it was thermally throttled or out of
memory, which is exactly the difference §3.8 requires MCF to know between "this
model is slow" and "this machine was busy". DEC-008 has to be decided knowing
that the interesting fields are behind a C ABI.

**The cost was one module and no dependency.** `dlopen`/`dlsym` declared
directly, eight symbols resolved, every pointer null-checked and every status
code checked before its out-parameter is read. `unsafe` is confined to that
module; everything above it sees safe Rust and a classified failure. The
workspace denies `unsafe_code` as a `deny` rather than a `forbid` for exactly
this, and the opt-in carries its reason at the site.

**A boundary this raises for DEC-008 rather than settles.** Intent v23 defers
D23's platform-provided tier for *inference*, on the ground that an unpinned
runtime is an unpinned variable in every result taken through it. Reading a
driver's report of its own state is a different act: nothing MCF publishes is
computed through the library, so there is no result for it to be a variable in.
That distinction is recorded here, not adopted.

### 1.3 The budget is met with room, and it is missing a statistic

| Quantity | D24 | Measured | Verdict |
|---|---|---|---|
| Artifact on disk | ≤ 40 MiB (core binary) | 433 120 B | pass, by two orders of magnitude |
| Resident, nothing loaded | ≤ 20 MiB | ≈ 7.6 MiB | pass |
| Cold start to first response | ≤ 100 ms | median 3.9 ms, p95 9.6 ms, n = 20 | pass |

The subject of the cold-start figure is `mcf --version` — the shortest complete
command MCF has — measured as the whole round trip a shell sees.

**The finding is the statistic, not the numbers.** D24 states *cold start to
first command response ≤ 100 ms* and does not say which statistic that is. On a
quiet machine it does not matter: median 3.9 ms and p95 9.6 ms are both far
under. On a machine that was simultaneously compiling this repository, the same
twenty trials gave a median of 7.2 ms and a **p95 of 165–257 ms** — a passing
median and a failing tail, from one run. D24 specifies "p99" for added latency
and says *the tail is what a user feels*; it specifies nothing for the other
fifteen. §7.50 records the gap and DEC-050 tracks it.

The contended reading is also a demonstration of B35 rather than a defect: a
timing taken under contention measures the contention. B-011's suite has to
either open an exclusive window or mark its results unattributable (B24), and
this run is the evidence that the difference is an order of magnitude rather
than a rounding error.

### 1.4 What the prototype got wrong, which is why it exists

Its first draft measured its own cold start by re-executing itself, and did not
terminate. B18 makes a bug a fixture before it becomes a fix; this one is
recorded here because the lesson generalizes past the bug — a self-measuring
instrument measures itself measuring itself, and B-011's suite will face the
same shape when it measures the daemon's idle cost from inside the daemon.

### 1.5 What is still not measurable

Idle CPU, timer wakeups, memory growth over thirty simulated days and added
request-to-first-token latency are all D24 figures about a **daemon**, and there
is no daemon until M2 (B-030, B-031, B-035). They are reported as *not
measurable at this stage* rather than estimated, because A20 forbids an estimate
that could be read as a measurement and A7 forbids a plausible substitute for
one.

## 2 · F2 — The development machine cannot attribute a budget (DEC-051)

**What was run.** The budget tier (B-011), in release, on the machine MCF is
being written on.

**Conditions.** The same machine as F1: 16 cores, 32 threads. Its one-minute
load average sat between 44 and 47 for the whole session, from work that has
nothing to do with MCF — a language server pool and a long-running application
belonging to the operator. That is not an artefact of measuring; it is what the
machine is.

**What happened.** Every event-class figure came back **unattributable**, which
is what D27 says should happen and is the correct answer. Alongside it, the
readings themselves:

| Figure | Reading | Ceiling | Verdict |
|---|---|---|---|
| Core binary | 548 776 B | ≤ 40 MiB | within |
| Resident memory | 7 655 424 B | ≤ 20 MiB | unattributable |
| Cold start, median of 100 | 8.7 ms | ≤ 100 ms | — |
| Cold start, **p99** of 100 | **42.3 ms** | ≤ 100 ms | unattributable |

The median and the p99 differ by a factor of six on a machine that is simply
being used. F1 saw the same shape at a factor of twenty-five under a compile.
D27's rule — that the tail is what a budget is about — is doing real work here:
a median-based budget would have reported a machine six times better than the
one the operator has.

**The finding is that the tier can assert exactly one figure here, and this is a
gap rather than a defect.** The binary's size is asserted, because a file's
length is read as a single value and does not consult attributability at all —
nothing else running can change it. Everything that goes through a measurement
comes back unattributable, event-class and state-class alike, because D27's rule
is about the run rather than about the kind of figure.

There is no threshold that would fix the event-class half. The machine really is
busy, and a timing taken on it really does measure the contention (B35);
loosening the rule until the reading passed would be choosing the answer.

The state-class half is a smaller and more answerable question, and it is
recorded rather than assumed: a fresh process's resident set is affected by
memory pressure and not by CPU contention, so it is not obvious that the load
average should gate it. D27 did not distinguish, and §7.51 now asks whether it
should.

So MCF currently has no way to assert an event-class budget on a machine
somebody is using, which is most machines. B35 already names the mechanism that
would — an exclusive window, announced, bounded and interruptible — and it is
M6 work (B-181, B-182) built for measuring *models*. Whether MCF's own budgets
should open one, and what a scheduled tier does on a CI runner that is never
quiet, is **DEC-051**.

**What is not in doubt.** The tier is not reporting a failure and calling it a
success, nor the reverse. It reports the readings, states that it is not judging
them and why, and does not refresh its own age (B38). The dishonest outcomes are
the ones this design forecloses.

## 3 · F3 — The load average answers the wrong question, and the obvious fix silently could not fail (DEC-051, D30)

**What was run.** Two experiments, after F2 established that MCF could not
assert an event-class budget on the machine it is written on.

**Conditions.** The same machine as F1 and F2: 16 cores, 32 threads. The load
this time was produced deliberately — thirty-two spinning shell loops, bounded
to the duration of each measurement — because the operator's own workload had
finished and a quiet baseline was available for the first time.

### 3.1 The load average is on the wrong time scale

A hundred `mcf --version` cold starts, quiet and under load, with the measuring
process's own scheduling delay read across the whole measurement:

| | quiet | under load |
|---|---|---|
| Cold start, median | 208 µs | 1 041 µs |
| Cold start, **p99** | **360 µs** | **4 822 µs** |
| Wall time for the whole measurement | 21.8 ms | 139.1 ms |
| Own runqueue wait | 4 148 ns | 15 477 083 ns |
| **Wait as a fraction of the measurement** | **0.019 %** | **11.1 %** |
| **One-minute load average** | **0.29** | **0.29** |

The p99 moves by a factor of thirteen. The load average does not move at all —
not because it is imprecise, but because a **one-minute average cannot answer a
question about a 140-millisecond measurement**. By the time it responds, the
measurement is long over. The scheduling delay moves by a factor of 585 and does
so inside the window it is describing.

This is what D30 rests on, and it also corrects an assumption F1 and F2 were
reasoning under. The cold-start figures those recorded — 6 to 8 ms median — were
themselves contaminated: on a genuinely quiet machine the median is **208 µs**,
about thirty times faster. Every one of MCF's budget figures had been measured on
a machine that was never quiet, and none of them was wrong about passing, but all
of them were wrong about the number.

### 3.2 The obvious implementation could not fail

The first implementation read `/proc/self/schedstat`. It reported
**0.0000 % in both states** — quiet and under thirty-two spinners — which is
exactly what F2's failure looked like from the other side, and it was found by
the negative control rather than by review.

The cause: a process's accounting is its **main thread's**. Every measurement MCF
takes under a test harness runs on a worker thread, and the main thread is
blocked waiting for it, so it never queues. Reading `/proc/thread-self/schedstat`
instead:

| Same 300 ms of work, on a worker thread | quiet | under load |
|---|---|---|
| `/proc/self/schedstat` — the process | 0.0000 % | **0.0000 %** |
| `/proc/thread-self/schedstat` — the thread | 0.0207 % | **50.46 %** |

**The finding is about method as much as about the reading.** A signal that
cannot fail is not a signal, and this one silently could not. What caught it was
running the budget tier under deliberate load and checking that the verdict
*changed* — the negative control B-003's lint check had already established as
necessary, applied to a measurement instead of to a lint. A check that has only
ever been observed to pass has not been observed.

### 3.3 What changed as a result

Every budget figure is now asserted on this machine, quiet, and correctly
refused under load:

| Figure | quiet | under thirty-two spinners |
|---|---|---|
| Core binary | 617 360 B — within | within (unaffected by load) |
| Resident memory | 7 696 384 B — within | within |
| Cold start, p99 | 575 µs — **within** | 21.8 ms — **not attributable** |
| Record write, p99 | 3 446 ns — within | — |

DEC-051's deadlock is gone rather than traded off: B38 and D27 both still hold,
and the tier refreshes its age whenever a reading was clean.

## 4 · F4 — What the new tiers found on their first runs (B-191)

**What was run.** The five tiers B-191 built — property, whole-system, fuzz,
load, soak — and the mutation runner, each on its first pass over the tree they
were written against.

**Conditions.** The same machine as F1–F3: 16 cores, 32 threads, 91 GiB. Debug
profile, because these are tests rather than measurements of the artifact;
`/tmp` is tmpfs, which matters for every figure below that involves a file. No
timing figure appears here: A18 keeps a throughput assertion out of a test tier,
and these tiers assert counts and invariants rather than speeds.

**Why this is a finding and not a commit message.** Four of the six things below
are about *MCF*, and three of those were not visible to any tier that already
existed. The fifth and sixth are about the tiers themselves, and D10's whole
argument for mutation testing is that a suite is a thing that has to be checked.

### 4.1 A condition did not round-trip through the record

The property tier's first run falsified `a_condition_floor_survives_the_record_including_its_unknowns`
at its first case. `encode::conditions` rendered every known condition through
`Display`, so `ConditionValue::Integer(4096)` — a context length, which is the
one §3.4 floor question that is naturally a number — was written as `"4096"` and
read back as `Text("4096")`.

Nine of the ten floor questions never noticed, because nine of them are
naturally strings, and B-007's round-trip test used a floor captured from a live
machine where the numeric question is `Unknown`. That is exactly the gap a
property closes: the existing test asked about the floor MCF happens to produce
today, and the property asks about every floor MCF can represent.

B-007's stated condition — *the §3.4 floor round-trips through the record store
losslessly* — was therefore false, and had been since the capture path was
written. Fixed, with the failing case kept as a fixture.

### 4.2 A replay holds the whole journal, and it costs about a kilobyte an entry

The soak tier, over a hundred thousand appends:

| | |
|---|---|
| Growth of the writer over 100 000 appends | **0 bytes** |
| Resident held by a replay of the same journal | ~100 MB |
| Per entry, for an entry with one integer field | **~0.8–1.0 kB** |

The first number is the leak check and it is asserted. The second is
proportional by construction — `Replay` returns every entry it read — and it is
D20's design rather than a defect: the journal is the record, and the index over
it is derived. It is **reported and not asserted**, because asserting on it
would be asserting that MCF never keeps a record long enough to matter.

It is worth keeping because it puts a number on why D6's derived index exists
(B-042, B-300): at a kilobyte an entry, a machine that has run a few million
trials cannot answer a question by replaying its journal into memory.

### 4.3 A process-wide reading cannot be taken beside another test

The soak tier reported a 70 MB leak in the journal writer. There is no leak: the
harness runs tests in parallel by default, resident memory is a property of the
*process*, and the reading had another test's replay in it.

This is B35's rule — *a reading taken under contention measures the
contention* — arriving at memory instead of at a timing, and the resolution is
the same one D30 reached: separate the reading from the thing that is not being
measured. The tier runs on one thread, and measuring the writer and measuring a
replay are two tests rather than two readings inside one.

### 4.4 The fuzz campaign reaches the accept path

Four parsers, 200 000 cases each by default, nothing found. That sentence is
worth very little on its own — a campaign that refused every input would say the
same thing — so each target counts what it reached:

| Target | Damaged inputs accepted | Refused |
|---|---|---|
| `json::parse` | 11 484 | 89 623 |
| `time::Zone::parse` | 20 637 | 179 363 |
| `journal::replay` | 2 916 | 2 084 |
| `export::read` | 181 | 4 819 |

The two that write files run a fortieth of the cases, which is why their totals
are smaller. `export::read`'s acceptance rate is low by construction: a bundle
states a digest over its own contents, so most damage is refused by the check
the reader exists to make. A target that never accepted anything would fail the
tier rather than pass it.

### 4.5 The mutation runner was judging mutants against the mutants before them

The first working run of `scripts/check-mutants.sh` reported the `export` mutant
as having hung. It does not hang: run alone, a unit test kills it in a tenth of
a second.

What hung was the **digest** mutant, still compiled in. Cargo decides what to
rebuild from modification times, and the runner restored each file by moving
back a backup taken *before* the mutation — so the restored file was older than
the build made from it, cargo considered the crate fresh, and did not rebuild.
The consequence is more general than the symptom: every mutant after the first
was judged against a tree that still carried the earlier mutations, wherever
those lived in a crate that had no other reason to be recompiled. A mutation in
`mcf-core` survived a later mutation in `mcf-record`, because nothing asked for
`mcf-core` again.

That direction of error inflates a score rather than deflating it — extra
breakage makes a mutant easier to kill — which is the kind that a green run
hides. The fix is one `touch` on the restored file.

The reason it took three runs to find is the part worth keeping. The
equivalent-mutant control runs **first**, before anything has leaked, so it
cannot see this class of defect at all. What sees it is a comparison of the
whole copy against the tree at the **end** of the run, which the runner now
makes and which refuses to report a score if a restore did not take. A control
at the start of a run and a verification at the end answer different questions,
and this tier needed both.

### 4.6 The score, and the mutant that hangs

Eleven mutants, each breaking something a rule depends on:

| | |
|---|---|
| Killed | **11** |
| Survived | 0 |
| Invalid (did not compile) | 0 |
| Of those killed, by hanging | 1 |

A 11-of-11 score is not evidence that the suite is complete; it is evidence
about eleven specific claims, chosen because a rule rests on each. What it does
establish is that the tier itself works — the control is not killed, a mutant
that does not compile would be excluded rather than counted, and the run refuses
to report at all if the copy is not the tree again afterwards.

The hanging mutant is the digest one: it takes the room left in a 64-byte buffer
from 64 to 63, so the filling loop eventually takes zero bytes per pass. Every
suite run is bounded, a timeout is confirmed by a second run before it is
believed, and what a hung run leaves behind is reaped — a spinning test process
outliving the tier is a change to the machine A27 does not permit MCF's own
suite to make either.

## 5 · F5 — The cold-start budget is a measurement of the filesystem (B-011, D30)

**What was run.** `scripts/ci.sh --all`, the first invocation that runs every
tier B-191 declares. The budget tier failed: cold start, p99 of 100 trials,
**252 ms** against D24's ceiling of 100 ms, and the reading was judged
*attributable* rather than refused. A later run of the same command, on the same
machine, passed the same figure at 0.58 ms — which is 5.3 below, and is the part
that settles it.

**Conditions.** The same machine as F1–F4. The one condition that turned out to
matter is one nothing was recording: **the repository lives on
`/home/gauge/Content`, which is a `fuseblk` mount**, and the artifact under test
is executed from there. `/tmp` is tmpfs.

### 5.1 The same binary, two filesystems

A hundred spawns of `mcf --version`, release profile, from each location, timed
outside MCF to keep the instrument out of its own finding:

| The binary is read from | median | p99 | minimum |
|---|---|---|---|
| `fuseblk` — where the repository is | 0.362 ms | **416.081 ms** | 0.252 ms |
| `tmpfs` — a copy of the same bytes | 0.222 ms | **0.390 ms** | 0.212 ms |

The medians differ by a factor of 1.6. The **p99s differ by a factor of 1 067**.
The same file, the same machine, the same instant: what differs is the
filesystem the kernel faults the pages in from, and a FUSE filesystem
occasionally takes hundreds of milliseconds to serve one.

D27's choice of the 99th percentile is doing exactly what it was chosen to do —
*the tail is what a user feels* — and what it caught here is real. It is simply
not about MCF.

### 5.2 Two things this says, and neither is that the budget is wrong

**The storage an artifact is executed from is a measurement condition, and MCF
does not record it.** §3.4's floor asks ten questions and none of them is *where
did this come from*. Two runs of one binary, with identical stated conditions,
differ by three orders of magnitude in the statistic D24 is written in. A reader
handed both numbers could not tell which was which, which is precisely what A6
exists to prevent.

**D30's attributability signal cannot see this, by construction.** It reads the
*measuring thread's* time on the runqueue: the question "was this reading
affected by contention for the processor" (F3). During a cold-start measurement
the measuring thread is blocked in `wait4` and is not runnable at all, while the
child faults its pages in from a slow filesystem. The delay signal stays near
zero and the reading is judged clean, which it is — of the thing the signal
measures.

So MCF has a signal for one kind of contamination and no signal for another, and
the tier's only event-class figure happens to be dominated by the second. That
is a gap in the instrument rather than in the number: B-193 registers both
halves, and until it is built the cold-start figure means *what a cold start
costs on this storage*, which is worth knowing and is not what D24 asked for.

### 5.3 The same tier passes and fails on the same machine within the hour

Run again twenty minutes later, with the mount's page cache warm from the run
before it, the tier was green:

| Run | Cold start, median | Cold start, p99 | Verdict |
|---|---|---|---|
| After the mutation tier had written ~10 GB to tmpfs | 152.2 ms | 252.4 ms | **over** |
| With the mount's cache warm | 0.370 ms | 0.582 ms | within |

A factor of four hundred on the median, between two runs of one command on one
machine, with every condition MCF records identical. That is the sharper form of
what 5.2 says: the figure is not merely mis-attributed, it is **not
reproducible**, and P3
puts reproducibility above convenience. A budget that passes or fails according
to what else has been evicting the page cache is not yet a budget.

It is also why the fix is two halves rather than one. Recording the filesystem
would make the two runs legibly different; only the attributability half makes
the second one *refuse* rather than report.

### 5.4 What this does not change

The tier is behaving as designed in the part it can see: the state-class figures
assert, the reading was reported with its statistic and its sample count, and
the failure was loud. B-011 stays in progress, and it now has a stated reason
beyond the missing baseline.

Nor does it change what B-191 established. `scripts/ci.sh --all` ran all ten
tiers; nine of them were green and the tenth failed on a real reading, which is
the outcome a tier exists to produce.

## 6 · F6 — The first mutant to survive (B-186)

**What was run.** `scripts/check-mutants.sh` against a copy of the tree carrying
a twelfth mutant, added to check that B-186's floor actually refuses. The mutant
was chosen because it looked likely to survive: `resident_bytes` multiplies the
kernel's kibibytes by 1024, and the mutant multiplies by 1000.

**Conditions.** The same machine as F1–F5, debug profile, the whole workspace
suite as the judge.

**It survived.** Eleven of twelve killed, 91 %, and the two refusals fired
exactly as intended — below the floor, and below the previous run's score.

### 6.1 What the survivor was hiding

Nothing checked that MCF's resident-memory reading is in bytes. The existing
test asserted a plausibility band — greater than zero, less than 64 GiB — which
a figure 2.4 % wrong passes without difficulty.

That figure is not decorative. `mcf doctor` prints it, and B-011 asserts it
against D24's 20 MiB ceiling. A19 requires that anything reported be tested
against an independently known value, and this one was tested against itself.

The fix is two independent facts about the same quantity: the kernel reports
`VmRSS` in kibibytes, so the value in bytes is a whole number of pages; and
`/proc/self/statm` counts the same pages in a different file. A mutant
multiplying by 1000 lands off the page boundary and dies.

### 6.2 What it says about the number 100 %

The floor is a hundred per cent of a hand-written catalogue, which is a much
weaker claim than a hundred per cent of everything a generator could produce —
and it is the claim worth making. Each entry breaks something a rule in
rules.md rests on, so the score answers *are these twelve claims checked*
rather than *what fraction of arbitrary edits does the suite notice*.

The loop the tier exists for ran in full here: a mutant survived, the claim it
broke got a test, and the mutant joined the catalogue. A tier that reported 91 %
and moved on would have left the same gap with a number attached to it.

## 7 · F7 — A major page fault is the signal F5 was missing (B-193)

**What was run.** Thirty spawns of one binary, twice: once with the file
resident, once with its pages evicted from the cache before each spawn, counting
the major page faults the kernel charged to this process's children.
`scripts/check-fault-signal.sh` is the experiment, and it runs in the gating
tier.

**Conditions.** The same machine as F1–F6. The probe ran from
`$XDG_CACHE_HOME`, which is btrfs; the repository's own filesystem is a FUSE
mount that does not honour the eviction hint, which is why the script tries
several directories and names the one that worked.

### 7.1 The reading

| | Major page faults over 30 spawns |
|---|---|
| Warm — the file resident | **0** |
| Evicted before each spawn | **30** |

One per spawn, exactly, and none at all when the file is where a measurement
wants it. That is what makes the threshold **zero** rather than a judgement like
`TOLERATED_DELAY_PPM`: a major fault is the kernel going to a device, and a
measurement that took one waited on that device.

### 7.2 Why the other signal could not see it

D30's signal reads how long the *measuring thread* was runnable and not running.
During a cold-start measurement that thread is blocked in `wait`: it is not
runnable, so it accrues no delay, and the reading comes back clean however long
the child spent faulting its pages in. F5 is that blind spot with a number on
it — a figure two and a half times its ceiling, judged attributable.

The two signals are not a refinement of one another. They answer different
questions — *was this thread queuing for a processor* and *did this work go to a
device* — and a measurement can fail either. Where both hold, MCF reports the
device, because a busy machine is somebody else's compile finishing and an
artifact that was not resident is a property of where it lives.

### 7.3 What the pair now does

A cold start on slow storage is **refused as unattributable** rather than
reported as over its ceiling, which is what B-193 asked for. Beside it, the
storage the artifact was read from joined the condition floor as its eleventh
question, so the two runs 5.3 above compared — 152 ms and 0.37 ms on one machine
within the hour — are now legibly different rather than mysteriously so, and the
budget tier refuses to compare a reading with a baseline taken from different
storage (A8).

**What is still true and unfixed:** a p99 over a hundred trials moves by about a
quarter between runs on an idle machine, so the cold-start figure is still
recorded rather than judged against its baseline. The ceiling judges it; the
tolerance would fire on the tail.

## 8 · F8 — How far a kernel MCF could maintain is from a specialist's (DEC-004)

**What was run.** `prototypes/kernel-slope`: one 512×512 single-precision matrix
multiply — the operation an inference engine spends nearly all of its time in —
four ways, each of them something MCF could actually write and maintain in safe,
portable Rust. Then the same multiply through a tuned BLAS present on the
machine, single-threaded, for the other end of the slope.

**Conditions.** The same machine as F1–F7, release profile, five trials per
variant, operands generated from a fixed seed so two runs are comparable. Every
variant's product is compared bit-for-bit against the definition before its time
is kept: a kernel that is fast and wrong is not a data point (A19). The machine
was **not** quiet — a one-minute load average of about 34, from an editor's
indexers — and the single-threaded readings came back attributable at around
0.1 % queuing while the threaded one did not, which is D30 doing its job.

### 8.1 The readings

Medians of three runs of the prototype:

| Variant | Median | Against the definition |
|---|---|---|
| Naive — the definition, in the obvious loop order | 226–270 ms | ×1 |
| Reordered — the same arithmetic, loops walking memory forwards | 53–86 ms | ×3–5 |
| Blocked — tiled to keep a working set in cache, written carefully | 126–143 ms | ×1.6–1.9 |
| Blocked and threaded — the same across 32 threads | 19–27 ms | ×9–14 |
| **A specialist's kernel** — OpenBLAS, **one** thread | **1.5–2.5 ms** | **×100–170** |

Two ratios matter and both are measured here rather than assumed:

- The best MCF could do **single-threaded** is 53–86 ms against 1.5–2.5 ms.
  **A specialist's single core is twenty-five to fifty times MCF's best.**
- MCF using **all thirty-two threads** is 19–27 ms, still **about ten times
  slower than one** of that specialist's cores.

### 8.2 The second step of tuning made it worse

The blocked variant — the careful one, the one that looks like optimization — is
consistently **slower** than the one-line loop reorder. Cache blocking without
operand packing and without a register-blocked microkernel adds loop overhead
and address arithmetic for a locality benefit the reorder had already collected
at this size.

That is not a defect in the prototype; it is the finding. From the bottom of
this slope, the *sign* of an optimization is not obvious, and getting it right
means measuring each step on each machine — which is the treadmill §7.4 was
weighing, seen from the first rung.

### 8.3 What the remaining distance is made of

The gap to the specialist is not algorithmic. It is hand-written SIMD
microkernels per instruction set, operand packing into contiguous panels,
prefetch scheduling, and a different code path per generation of processor.
OpenBLAS reports itself here as `DYNAMIC_ARCH Haswell` — a *generic* kernel,
not one tuned for this processor — and it is still fifty times MCF's best.

None of that is reachable from where MCF stands. The workspace denies
`unsafe_code` for the reason §3.16 gives, portable SIMD is not in the stable
standard library, and every new processor and accelerator moves the target.
Owning it would mean owning it for ever.

### 8.4 What this settles

§7.4 stated the likely answer — *MCF's performance mandate applies to MCF's own
overhead, not to the inference kernels* — and refused to settle on a reading
alone. This is the measurement that reading needed, and it points the same way
by one to two orders of magnitude. D32 is the decision.

It also bounds the cost of the *other* half of that architecture. MCF's own
overhead, measured on this machine, is a cold start of a few hundred
microseconds and a record write of a few microseconds (F3, D24). A single 512³
multiply on a specialist's kernel is 1.5 ms — and a real model's forward pass is
thousands of those. Whatever MCF's wrapper costs, it is not where the time goes,
which is exactly why §VII's mandate belongs there and nowhere else.

## 9 · F9 — What a network costs, and what the hub actually does (B-021)

**What was run.** `prototypes/transport-cost/measure.sh`, on 2026-08-25: ask
the real hub for sixteen bytes of a real model over HTTP/1.1 and keep the
headers, then build the smallest program that could do the same thing in three
shapes and measure what each drags in and what it demands of a machine.

**Conditions.** The machine of F1. Rust 1.98.0, release profile, `cargo vendor`
against crates.io as it stood on the day. Everything was built outside this
repository: measuring a candidate is not admitting one. The machine was not
quiet — two other projects held the exclusive window — which does not matter
here, because nothing timed is reported.

**Why it was run.** Four items sat behind the same sentence: *what remains is
the transport*. B-021 has a fetcher with no socket, B-213 has arithmetic with
no metadata to do it on, B-024 has credentials nothing offers, B-029 has two of
three commands. MCF's tree has no third-party code at all, the hub speaks
HTTPS, and no decision had crossed that boundary. An argument about it would
have been an argument about numbers nobody had.

### 9.1 The hub is simpler than feared, and says more than expected

Every line below is a requirement on whatever MCF writes, and every one of them
was a guess beforehand.

| What was asked | What came back | What it settles |
|---|---|---|
| `--http1.1` | `HTTP/1.1 206` | **No HTTP/2 is needed.** The whole h2 stack — framing, HPACK, flow control — is off the list |
| A model file by its repository path | `302` to a signed URL on another host (`us.aws.cdn.hf.co`) | Redirects cross hosts, so a client must follow them **and must not carry the credential across** |
| `Range: bytes=0-15` | `206 Partial Content`, `content-range: bytes 0-15/396705472` | Resumption works, and B-021's `fetch_from` has a real counterpart |
| The same request, headers kept | `x-linked-size: 396705472`, `x-linked-etag: "ac2d977…d524a"` | The **declared size and SHA-256 arrive before the bytes do** — `Entry::declaring` is what the hub already offers |
| The same | `x-repo-commit: 50968a44…` | The revision to pin at acquisition (B-019) is in the response to the download itself |
| Nothing (no credential) | `x-hf-warning: unauthenticated`, `ratelimit-policy: "fixed window";"resolvers";q=3000;w=300` | Throttling is *published*, so `Behaviour::RateLimited`'s hint is real rather than invented |

### 9.2 The cost is cryptography, not protocol

Three shapes, each the smallest program that does the job:

| Shape | Crates | Vendored tree | Not Windows | Rust source | Artifact demands |
|---|---|---|---|---|---|
| A client and its TLS (`ureq`) | 42 | 91 MiB | 19 MiB | 31 MiB | `libc`, `libgcc_s`, the loader |
| TLS only (`rustls` + `ring` + roots) | 27 | 87 MiB | 15 MiB | 28 MiB | the same |
| The machine's own TLS (`native-tls`) | 5 | under 1 MiB | — | — | **`libssl.so.3`, `libcrypto.so.3`** and the three |

Three things fall out of that table.

**Most of a vendored tree is Windows import libraries a Linux build never
compiles.** 72 of the 91 MiB are `windows-sys` and its target crates. Reporting
the headline number would have overstated the cost fivefold, which is how a
real objection gets dismissed for the wrong reason. The honest figure for the
TLS-only shape is **sixteen crates and 15 MiB**: `cc`, `cfg-if`,
`find-msvc-tools`, `getrandom`, `libc`, `once_cell`, `ring`, `rustls`,
`rustls-pki-types`, `rustls-webpki`, `shlex`, `subtle`, `untrusted`, `wasi`,
`webpki-roots`, `zeroize`. Six of those are a few hundred lines each.

**The HTTP client is the cheap half.** Fifteen crates and 4 MiB separate *TLS
only* from *a client and its TLS*. So the question is not whether to write an
HTTP client — that saves little and is easy to test — it is whether to vendor
cryptography at all, and there is no third option: 9.1 says the hub answers
nothing that is not TLS.

**The cheapest thing to vendor is the one thing MCF may not ship.** The
system-TLS shape needs almost nothing and produces a binary that demands
`libssl.so.3` and `libcrypto.so.3` — two libraries [vendored.md](vendored.md)
§3a does not list, whose versions differ between distributions, and which B36
would make the user's errand. It is in the table as the control: the shape that
fails, failing where a check can see it rather than on somebody's machine.

### 9.3 What the byte counts do not show

`ring` is C and assembly, so vendoring it puts **a C compiler in MCF's build**
— visible in the tree as the `cc` crate. The artifact does not gain a
dependency (row one of the table is the evidence), but the *build* gains a
prerequisite, and §3.12's reproducibility claim would then rest on a second
toolchain nothing pins. That is a cost to state rather than to discover, and
it is the strongest argument any pure-Rust provider has.

Against that: **MCF cannot write this.** The rest of the tree is code MCF could
in principle maintain; a TLS 1.3 implementation is not, and A19 forbids
claiming what is not tested. The alternative to vendoring cryptography is
having no network, and having no network is the end of §III.

### 9.4 The tree cannot be trimmed to the platform

The obvious answer to *72 of 91 MiB are Windows import libraries* is to vendor
only what this platform builds. Cargo will not have it: with a vendored source
directory it resolves the whole lock graph before it compiles anything, so a
tree with `windows-sys` removed fails to build on Linux — not because Linux
needs it, but because the resolution does.

Measured rather than assumed, and it changes the number that matters. Admitting
a TLS stack means **about 90 MiB of third-party source in the repository**, not
15. The 15 MiB is what a Linux build *compiles*, which is the right number for
reviewing what MCF ships and the wrong one for what a checkout costs.

That is a cost worth stating in one place rather than discovering in a diff, and
it is the reason B-322's remaining step is a deliberate admission rather than
another commit: everything else the transport needs is written and tested, and
what is left is one struct and a decision about ninety megabytes.

### 9.5 The obvious provider costs a claim MCF already makes

`rustls`'s usual cryptography is `ring`, which is C and assembly. Building the
`tls-ring` shape for `x86_64-unknown-linux-musl` — the target
`scripts/check-from-scratch.sh` uses to put MCF in a container with *nothing*
in it — fails on this machine:

```
error occurred in cc-rs: failed to find tool "x86_64-linux-musl-gcc"
```

So admitting `ring` does not merely add a C compiler to the build (9.3): it adds
a **cross** C toolchain, without which B-183's from-scratch check cannot run at
all. The check already distinguishes *could not be checked* from *passed*
(exit 2), so nothing would be silently lost — but a claim that can only be
tested where a particular toolchain is installed is a claim that gets tested
less.

### 9.6 A provider MCF can build anywhere, and what it costs

`rustls-graviola` is a provider written in Rust and inline assembly, by
`rustls`'s own author. Measured against the same questions:

| | `ring` | `rustls-graviola` |
|---|---|---|
| Crates compiled here | 16 | **14** |
| Vendored tree, filtered to what compiles | 22.9 MiB | **13.0 MiB** |
| C or assembly *source files* in the tree | 93 | **0** |
| Builds for `x86_64-unknown-linux-musl` | no, without a cross toolchain | **yes** |
| A TLS 1.3 session to the real hub | — | **yes**: `HTTP/1.1 200 OK`, 9937 bytes, `TLSv1_3` |

The last row is the one that matters and it was run rather than assumed: a
sixty-line program using `rustls` over a plain socket, against
`huggingface.co`, returning the same listing 9.1 fetched with `curl`.

**And the tree can be filtered after all.** 9.4 said a vendored tree cannot be
*trimmed*, and that is true of deletion — cargo resolves the whole lock graph.
It is not true of **stubbing**: a crate that no target MCF builds ever compiles
can keep its manifest, its licence files and an empty `lib.rs`, with a checksum
file that lists no files. Both the glibc and musl builds then succeed
`--offline --locked`. That is what `scripts/vendor.sh` does, and it is why the
number above is 13 MiB rather than 95.

**What this costs, stated rather than discovered.** `graviola` is young — the
mature choices are `ring` and `aws-lc-rs`, and both are C. MCF is choosing a
newer implementation to keep a check it already makes runnable on any machine,
and the exchange is worth stating: a weakness here is a weakness in what MCF
verifies a hub with. Two things bound it. The provider is one line behind
[`mcf_hub::wire::Wire`], so swapping it is a change to one struct rather than to
the acquisition path. And the digest MCF checks the bytes against arrives with
the listing rather than with the file, so a source that substitutes weights has
to substitute both — which does not make TLS optional, and does mean TLS is not
the only thing standing there.

[`mcf_hub::wire::Wire`]: ../crates/mcf-hub/src/wire.rs

**Verdict: the transport is TLS-shaped.** The evidence says vendor the
cryptography and own the protocol — the same shape D32 settled for inference,
for the same reason: delegate what specialists maintain, own the wrapper that
has to be correct. What MCF writes is the HTTP/1.1 client, because 9.1 shows it
is small, and because the two behaviours that matter — a redirect that must not
carry a credential, and a resumption that must verify — are precisely the ones
the laboratory has to be able to simulate.

Admitting a component is [vendored.md](vendored.md)'s business and B-322 is
where it happens. This finding is the stated reason B15 requires.

## 10 · F10 — What a machine says when a network is missing (DEC-011, D33)

**What was run.** A sixty-line program on 2026-08-25: resolve a name that
exists, resolve one that cannot, and connect to three addresses that answer in
three different ways. What is being measured is not the network — it is *what
the platform tells a program*, which is all MCF ever has.

**Conditions.** The machine of F1, on a working network, Rust 1.98.0. Nothing
timed is a measurement of anything but the platform's own paths.

| Asked | What came back | How long |
|---|---|---|
| Resolve a real name | an address | 60 ms |
| Resolve `this-name-does-not-exist.invalid` | `ErrorKind::Uncategorized`, no errno, *failed to lookup address information* | 0.5 ms |
| Connect to a closed port here | `ErrorKind::ConnectionRefused`, errno 111 | 0.2 ms |
| Connect to `203.0.113.1` (TEST-NET-3) | `ErrorKind::TimedOut`, no errno | the deadline |
| Connect to `10.255.255.1` | `ErrorKind::TimedOut`, no errno | the deadline |

### 10.1 A failed lookup is one observation with three causes

The row that decides DEC-011 is the second. A name that will not resolve
produces `Uncategorized` — **no error kind at all** — whether the cause is a
machine with no network, a machine with no resolver, or a name that does not
exist. The platform does not distinguish them and neither, therefore, can MCF.

That is not a gap to be filled by inference. MCF could *guess* by trying
something else — a second resolver, a known-good address, a ping — and each of
those is a network request nobody asked for, which §3.2 refuses and §3.13's idle
discipline refuses again. So the honest report is the observation: *this machine
could not turn that name into an address*, with the three causes named as what
the sentence does **not** say (A7).

### 10.2 Refused, no route, and silence are three different things

The other rows are worth keeping apart, because they are three different things
for a person to do something about. A refusal means something is there and said
no — there is a working path. No route means this machine cannot get there at
all, which is what an unplugged machine looks like from inside a program.
Silence means MCF's own deadline ended the wait rather than the far end, which
is a statement about MCF's patience and not about the network.

`mcf_hub::wire::Tcp` now says which of the three it saw, and the gating tier
holds two of them without a network: `.invalid` never resolves (RFC 2606), and
a closed loopback port is always refused.

### 10.3 What already works with nothing

The other half of §7.11's question needed no new experiment, because B-183's
from-scratch check answers it every time it runs: `--version`, `licence` and the
whole of `doctor` — the hardware profile, the self-cost measurement over a
hundred process spawns, and the laboratory reproducing every failure MCF claims
to handle — run in a container with **no network interface, no libc, no shell
and no `/etc`**. `list` and `rm` read and move local files and need nothing
either. The only command that needs a network is the one that fetches, which is
the answer §3.2 suggested and nobody had stated.

**Verdict: D33.** Offline is the ordinary case rather than a degraded one, and
what MCF says when a network is needed and missing is what it observed —
never which layer is absent. *No internet* versus *no local network* is a
distinction an operator draws by pointing `--from` at a mirror, which is a
request MCF was asked to make; it is not one MCF asserts by probing, which would
be traffic nobody asked for.

## 11 · F11 — A disk fills at the flush, not at the write (B-026)

**What was run.** Two writes to `/dev/full` on 2026-08-25 — a device every Linux
machine has that accepts everything and stores nothing, answering every write
with `ENOSPC`. One direct, one through a buffered writer.

| Written | What came back |
|---|---|
| `write_all` straight to the device | `ErrorKind::StorageFull`, errno 28, at the write |
| `write_all` through a `BufWriter` | **`Ok(())`** |
| the `flush` that followed it | `ErrorKind::StorageFull`, errno 28 |

**The middle row is the finding.** A small write into a buffer succeeds, because
nothing has reached the filesystem yet. A fetcher that checked its writes and
ignored its flush would have been told the truth and then discarded it — and
would go on to verify a digest over bytes that are not on the disk, rename a
file that was never written, and record an acquisition that did not happen.

MCF's transfer path flushes and classifies both, and `hub/no-room-on-the-disk`
is the scenario that holds it: the laboratory writes a transfer to `/dev/full`
and the outcome is `resource.disk.exhausted` rather than a success or a general
write failure.

**The other half of §3.11 is the number before the transfer.** A file that will
not fit is refused *before* a byte moves, with the arithmetic in the refusal —
what it needs, what is available, what it is short by, and which filesystem. The
platform has that number and the standard library does not expose it, so
`mcf_core::hardware::space` is the second module in the workspace to take the
`unsafe_code` opt-out: one `statvfs` call, a status checked before any field is
read, and `Unknown` wherever it fails, because *MCF could not look* and *there
is no room* are opposite answers (A7). The reading is checked against `df` — a
different program by other people (A12) — within a hundredth of the filesystem's
size, which is what two readings of a number in motion are worth.

**What it does not buy.** Certainty. The room is what the kernel said at the
moment it was asked, and another process can take it a moment later. That is why
both halves exist: the check turns the common case from a surprise into a
refusal, and the classification catches the case the check cannot.

## 12 · F12 — What an engine would cost, as far as it has been measured (B-320)

**What was run.** `prototypes/engine-cost/measure.sh` on 2026-08-25: fetch the
two candidates §XVI's vendoring constraint leaves standing, and measure what
each would be as a *thing to ship* — how much source, under what terms, needing
which toolchain.

**Why the question is not "which is fastest".** D32 settled that MCF delegates
inference; [vendored.md](vendored.md) asks for a compatibility finding before
anything ships; and B15 admits weight against a stated cost. Speed is what an
engine is *for* and is measurable later, on this machine, by the suite MCF is
building. What has to be known first is whether admitting one breaks something
MCF already promises.

### 12.1 What is measured

| | llama.cpp | candle |
|---|---|---|
| What it is | C++ and CMake, one upstream | Rust, no C++ toolchain |
| Licence | MIT | MIT or Apache-2.0 across 152 crates |
| Revision looked at | `d222767c7a651655` | crates.io as it stood on the day |
| Source MCF would ship | **35 MiB** — `ggml`, `src`, `common`, `include`, `vendor` | **119 MiB unfiltered**; the stubbing F9.6 uses would cut it, by how much is unmeasured |
| Lines of C or C++ | **464,000** | 94 C or assembly files inside otherwise-Rust crates |
| Third parties to account for | **one** | **152** |
| Build needs | CMake ≥ 3.14 and a C++ compiler | `cargo`, and a C compiler for a few crates |

The whole llama.cpp checkout is 167 MiB, of which `models/` and `docs/` are 110;
those are not what a vendored engine would carry, and quoting the headline
number would overstate the cost the way F9.2's 91 MiB did.

### 12.2 The shape of the trade, stated before it is made

**One upstream against a hundred and fifty.** llama.cpp is 464,000 lines of C++
from one project, under one licence, at one revision — a great deal of code and
a single thing to account for. candle is a hundred and fifty-two crates, each
its own project with its own terms and its own release cadence, and
[vendored.md](vendored.md) §2 would grow a row for every one. Neither is
obviously the smaller obligation: one is more code to read and less bookkeeping,
the other the reverse.

**The toolchain is where F9.5's lesson applies again.** This machine has `cmake`
and `g++` and **no musl cross toolchain**, which is exactly the condition that
decided the TLS provider: a candidate that cannot be built for
`x86_64-unknown-linux-musl` does not fail a test, it makes B-183's from-scratch
check runnable in fewer places. For a C++ engine that question is open and
serious; for a Rust one it is likely to be the same answer TLS got.

### 12.3 The deciding half, measured in the exclusive window

Both candidates were built. The numbers below were taken with `heavy` holding
the machine, because a build this size run beside somebody else's measurements
spoils them (B35, and section 12 of the build document).

| | llama.cpp | candle |
|---|---|---|
| Builds for `x86_64-unknown-linux-gnu` | **yes** | **yes** |
| What it produces | `libllama.a` 9.9 MiB + `ggml` 2.9 MiB = **12.9 MiB** of static libraries | a 512³ matmul program: **1.6 MiB**, needing only `libc`, `libgcc_s` and the loader |
| Builds for `x86_64-unknown-linux-musl` | not attempted: it is C++, and this machine has no musl C++ compiler | **no** — `onig_sys`, a C library, fails for want of `x86_64-linux-musl-gcc` |
| Crates in the graph | one project | **143**, for `candle-core` *alone* |

### 12.4 The musl question stops separating them

F9.5's lesson was that a candidate which cannot build for musl costs MCF a check
it already makes. That is what chose the TLS provider, and the expectation going
in was that it would choose the engine too — Rust over C++, for the same reason.

It does not, and the reason is worth recording. `candle-core` depends on
`tokenizers`, `tokenizers` depends on `onig`, and `onig` is Oniguruma — a C
regular-expression library. So the pure-Rust candidate needs a C cross toolchain
for the musl target exactly as the C++ one does. Neither keeps B-183's check
runnable on a machine without one, and the difference that decided TLS is not
available here.

**Read F18 after this.** *On a machine without one* is doing more work in that
sentence than it looked like at the time: the toolchain is packaged for this
machine and costs about nine megabytes. What F12 measured is unchanged; what it
concluded — that neither candidate keeps the check runnable — holds only where a
toolchain cannot be had, which turns out not to be here.

There is a second thing in that dependency worth noticing: `tokenizers` is
capability MCF already has. D31 gave MCF its own GGUF reader and its own
tokenizer so that the vendored engine would have something to be checked
against, and admitting `candle-core` would bring a second tokenizer along with
it — weight admitted for something already owned, which is what B15 asks a
reason for.

### 12.5 What this finding does and does not settle

It does not choose. What it establishes is the ground a choice would be made on,
and one expectation it removes:

- **Both build here.** Neither is blocked on this machine's toolchain for the
  ordinary artifact.
- **Neither builds for musl** without a C cross toolchain, so the from-scratch
  container either gains that prerequisite or ships without an engine — and *the
  second option is a difference in what two artifacts can do*, which §3.4 makes
  a condition of every measurement rather than a packaging detail.
- **The footprint is affordable but not free.** 12.9 MiB of static libraries
  against D24's 40 MiB ceiling is a third of it before a linker drops anything,
  and MCF's own binary is 3.3 MiB today.
- **The trade is one upstream against a hundred and forty-three.** That is a
  judgement about what MCF can account for rather than a number, and
  [vendored.md](vendored.md) §1 is where it would be argued.

B-320 is where the choice is made, and it is left open on purpose: this is the
largest single thing MCF will ship, and the register asks for a finding before
an admission rather than after.

## 13 · F13 — Two writers, one record (DEC-037)

**Why it was run.** The daemon exists, and with it MCF has more than one thing
that writes to the record: `mcf pull` records an acquisition, `mcf rm` a
removal, and the daemon its own starting and stopping. D20 makes the journal
*the record*, and §7.37 asks who writes to it and what happens to a write that
loses. Before deciding, it was worth knowing what the platform actually gives
without any coordination at all.

**What was run.** Eight processes appending to one file, each writing one whole
line per entry — the shape `mcf_record::journal` uses, which opens with
`O_APPEND` and issues one `write` per line. 2,000 lines each, at three sizes,
on the two filesystems that matter here: `tmpfs`, where a scratch record goes,
and `btrfs`, where the operator's own lives.

| Line size | Filesystem | Lines expected | Lines found | Interleaved | Short |
|---|---|---|---|---|---|
| 400 B | tmpfs | 16,000 | 16,000 | 0 | 0 |
| 8 KiB | tmpfs | 16,000 | 16,000 | 0 | 0 |
| 128 KiB | tmpfs | 16,000 | 16,000 | 0 | 0 |
| 8 KiB | btrfs | 16,000 | 16,000 | 0 | 0 |

**No line was torn**, at any size, including 128 KiB — far past `PIPE_BUF`,
which is the bound people usually quote for atomic appends and which applies to
pipes rather than to regular files. What the kernel guarantees here is that an
`O_APPEND` write takes the offset and the write together; a short write would
still tear a line, and none occurred.

### 13.1 What this settles and what it does not

It settles the alarming half. Two MCF processes writing one record do not
produce a record nobody can read: `checks/tests/load.rs` now asserts it —
concurrent writers, then a replay that reports no loss and finds every body
whole — and B62's *a replay reports what was lost* has nothing to report.

It does not settle DEC-037, and the residue is precise:

- **Identifiers can collide.** Each writer counts its *own* appends, so two
  entries from two processes can carry the same sequence number and therefore
  the same `EntryId`. Nothing is lost and nothing is unreadable; what is broken
  is the assumption that an identifier names one entry.
- **A short write would still tear a line**, and nothing here forces one. What
  was measured is that it does not happen on these filesystems at these sizes,
  not that it cannot.
- **A network filesystem was not measured.** `O_APPEND` is the classic thing NFS
  does not honour, and a record on a network share is a configuration MCF has
  not been asked about.

**What it argues for.** Not a lock. The measurement says the cheap arrangement
is sound where it has been tried, so the question is narrower than *how do we
coordinate* — it is *who mints an identifier*. A single writer answers it and
costs a running daemon for every command; a writer-scoped identifier answers it
and costs a change to something C5 makes stable for life. DEC-037 is where that
is chosen, and this is the evidence it should be chosen against rather than
guessed at.

## 14 · F14 — What a query over the record costs, and what an SQL engine would cost to ship (B-300, B-042, D6, D20)

**Why it was run.** D6 says the record is a SQLite database. D20, written later,
says the record is an append-only journal with a **derived, rebuildable** index
over it. Both can be true — one of them is the record and the other is the index
— but which is which decides whether MCF vendors 266,000 lines of C, and that is
not a question to settle by preference.

**What was run.** Two halves, one inside MCF and one outside it.

- `cargo test -p mcf-record --release --test how_the_record_grows -- --ignored`
  writes journals of 1,000 to 1,000,000 entries and times what MCF does with
  them.
- `prototypes/record-index/measure.sh` builds the same-sized table in SQLite,
  times two queries against it, and then measures the amalgamation as a *thing
  to ship*: source, terms, compile, and what it does to the musl artifact
  B-183's container runs.

Conditions: Linux 7.1.9, btrfs on NVMe, release profile, one machine, no
exclusive window — these are cost figures about MCF's own code and a candidate
dependency, not measurements MCF publishes (B35).

### 14.1 What a replay costs, and what an index costs instead

| Entries | Journal | Index | Full replay | Build the index | Open a current index | Last 20 of a kind |
|---|---|---|---|---|---|---|
| 1,000 | 0.4 MiB | 32 KiB | 6.4 ms | 10.7 ms | 128 µs | 183 µs |
| 10,000 | 3.8 MiB | 0.3 MiB | 98.7 ms | 103.9 ms | 812 µs | 318 µs |
| 100,000 | 38 MiB | 3 MiB | 777 ms | 988 ms | 9.1 ms | 230 µs |
| 1,000,000 | 381 MiB | 30 MiB | 7.89 s | 10.39 s | 72 ms | 196 µs |

A durable append costs **8.2 µs** and a whole entry is **400 bytes** on disk.
Replay is linear at about **8 µs an entry** — a parse and a `Value` per line.

Two numbers decide the shape. **7.89 s** is what a daemon start would pay to
count a million-entry record, at every start, and 381 MiB is what it would hold
to do it. **72 ms** is what opening the index costs instead, and **196 µs** is
what *the last twenty acquisitions* costs at any size, because the answer is
twenty seeks rather than a history.

A million entries is not hypothetical: M5 writes an entry per trial, and D16
keeps every trial rather than a summary.

### 14.2 What SQLite would buy, and what it would cost

Against the same million rows, through the `sqlite3` binary, including process
start each time:

| | |
|---|---|
| Build the table (WAL, one index) | 2.18 s |
| `count(*)` of one kind | 25 ms |
| Last 20 of one kind, by time | 16 ms |
| The database on disk | 151 MiB, beside the journal it was derived from |

And as a thing to ship:

| | |
|---|---|
| Source | 265,952 lines, 9.2 MiB, one file |
| Terms | public domain |
| Host object | 1.4 MiB, compiled in **51.6 s** by GCC 16.2.1 |
| Static musl artifact | **cannot be built here** — no C cross toolchain, the same wall F12 found for both engine candidates |

### 14.3 What this settles

**The index does not need to be a database, and D6's SQLite is weight MCF
cannot currently pay.** The queries a record actually gets — *the last twenty of
a kind*, *everything since a moment*, *how many of these are there* — are
answered in tens of microseconds by 32 bytes an entry, which is the same order
as SQLite answers them in and needs no C compiler, no cross toolchain and no
second copy of every body. The one thing SQLite would add that the index does
not have is *arbitrary* query — a `WHERE` over the bodies — and nothing in MCF
asks for one yet.

The cost side is not close. Admitting SQLite would put 9.2 MiB of C in the tree,
add 52 s to a cold build, and — on this machine, as it was configured when this
was measured — break B-183's from-scratch container, which runs a **static musl**
artifact with nothing installed. (F18 later found the missing toolchain is
packaged; the source size and the compile time are unaffected, and they are the
larger half of this.) F12 found the same wall for both engine candidates: the musl question
is turning into MCF's real constraint on vendoring, and it is a constraint about
a claim MCF makes rather than about a preference.

**What it does not settle.** SQLite is not refused for ever: if a query nobody
can answer with an offset table turns up, or the musl toolchain arrives with the
engine B-320 admits, this is a cost to pay rather than a rule to keep. What
changes today is D6's *the record is a SQLite database*, which becomes D20's
shape stated once: the journal is the record, and the index over it is derived,
32 bytes an entry, and free to delete.

## 15 · F15 — What actually needs elevation, asked of a machine (DEC-039, §7.39, §6.32)

**Why it was run.** §6.32 requires MCF's privileged surface be an enumerable,
auditable, short list, and §7.39 records that the list does not exist. It cannot
be written from documentation: what an ordinary user may do differs by kernel,
distribution, hardware and how the machine was set up. B-190's helper and
B-180's containment scenario are both blocked on the list, so it was asked.

**What was run.** `prototypes/elevation/measure.sh`, which **changes nothing**:
every row is a read, or a permission test of the kind `open(O_WRONLY)` performs,
or an operation on the probe's own process. A measurement that reconfigured a
shared machine to find out what it could reconfigure would be the ambient
authority §6.32 exists to refuse.

Conditions: Linux 7.1.9 (Fedora 44), uid 1000 in groups `wheel` and `ollama`,
cgroup v2 under a systemd user slice, one NVIDIA and one AMD device present.
**One machine.** Everything below is a fact about this configuration and a
hypothesis about any other, which is the whole reason DEC-039 asks *on which
platforms*.

### 15.1 What the machine said

| Operation | As this user | What wants it |
|---|---|---|
| Pin a process to cores | **permitted** | §6.39's ladder, B-193 |
| Bound this process's memory (`memory.high`) | **permitted** | §6.39, D8 |
| Read per-process accelerator occupancy | **permitted** | PR5, B-216 |
| Read temperature (thermal zone, hwmon) | **permitted** | §3.4's floor, B-013 |
| Raise priority (`nice -5`) | **refused** | §6.39's ladder |
| Real-time scheduling (`SCHED_FIFO`) | needs elevation | §6.39's ladder |
| Set the CPU frequency governor | needs elevation | §6.39, §3.25 |
| Disable SMT, offline a core | needs elevation | §6.39's upper rungs |
| Drop the page cache | needs elevation | a cold-start measurement |
| Set IRQ affinity | needs elevation | quieting a machine |
| Hardware performance counters | user-space events only (`perf_event_paranoid` = 2) | B-012 |
| **Read processor energy (RAPL)** | **needs elevation** | **D11 — energy is first-class** |
| Change accelerator compute or persistence mode | root, per the vendor's own tool | D8's exclusive lab |
| Lock memory | **8 MiB** (`RLIMIT_MEMLOCK`) | pinning weights |

**One row here was an absence rather than a fact** — *no C cross compiler on
this machine* — and F18 priced it: the toolchain is packaged. Nothing else in
this table changes.

**The one that costs something.** D11 makes energy a first-class quantity, and
`intel-rapl`'s `energy_uj` is not readable by an ordinary user on this
distribution — a deliberate change made after the counters were shown to leak
information about what a machine is doing. So *energy per token* is not a figure
MCF can take here without a privileged reader, and D11's claim needs either a
helper, a vendor counter that is readable (the accelerator's own power draw is,
through `nvidia-smi`), or an honest absence.

**Two that cost less than expected.** Core pinning and a memory bound need no
privilege at all: cgroup v2 delegates `cpu`, `io`, `memory` and `pids` to the
user's own slice, so MCF can ask for less of a machine without asking anybody.
And per-process accelerator occupancy is readable, so PR5's contention snapshot
— *what was I competing with* — does not need elevation.

### 15.2 The method has a lesson of its own

The first version of the probe reported that raising priority was **permitted**,
because `nice -n -5 true` exits zero. It does so having failed: the shell's
`nice` reports the exit status of the command it ran, not whether the priority
was applied. Asking the child what its priority actually *became* reverses the
answer.

That is A21 at the level of a shell script — a declaration is not an
observation — and it is the reason this file exists: a list of privileged
operations written from what the tools *said* would have been wrong on its first
row.

### 15.3 What this settles

It gives DEC-039 the list §6.32 asks for, on one platform, split by what it
costs. Four operations MCF wants need nothing. Two — a governor and real-time
scheduling — are the ladder's upper rungs and need a helper or must be dropped.
One — reading energy — is a *read* that needs elevation, which is the awkward
case §7.39 anticipated: a helper that must exist for a counter rather than for
an action.

**What it does not settle.** Every other platform. A Debian machine, a machine
without systemd, a container with no `/sys` write access at all, and macOS and
Windows are each their own answer, which is why DEC-039's answer has a column
per platform (D35) rather than a single list.

## 16 · F16 — Three defects the real reference model found, and none of them was the one expected (B-213, B-019, §3.7)

**Why it was run.** The operator asked which choices were still theirs, and one
of them was which quantization of the reference model to pin. MCF is supposed to
answer that without downloading anything (B-213, PR3), so instead of guessing at
file sizes the planner was pointed at the real repository.

**What was run.** `mcf pull unsloth/Qwen3.8-27B-GGUF` — the listing and the
plan, no bytes fetched — against the real hub, on the machine MCF is developed
on. Conditions: 61 GB of usable host memory, planning context 4096 tokens.

### 16.1 The plan could not be made at all, and the reason was a number

MCF said it could not plan and that *one of them could not be read*. The
repository publishes a `config.json` MCF's own reader refused, and the offending
text was:

```
"rms_norm_eps": 1e-06
```

MCF's JSON reader deliberately refused fractions and exponents. The reasoning
was sound and is still in the module: every quantity MCF *records* is integral
because A6's `Quantity` is `Ord`, and a record that silently rounded would be a
record that lied. What the reasoning missed is that **the same reader reads
documents MCF did not write**, and §3.7 makes those untrusted input rather than
MCF's own format. A reader that refuses valid JSON refuses real models.

The fix keeps both properties apart. A number this format does not carry is read
as `ForeignNumber`, **kept byte for byte**, and is not a quantity to anybody who
asks — `as_integer` is `None`. Nothing outside the reader constructs one, which
is what `checks/tests/no_float_reaches_the_record.rs` now holds. The record
stays integral because nothing writes anything else into it, rather than because
the reader cannot spell it.

### 16.2 The configuration was not where MCF looked

With the file readable, the shape still came back empty: this is a multimodal
repository, so one `config.json` describes several models and the transformer's
fields are under `text_config` while the top level holds the composition. MCF
now looks at the top level and then there — not a guess, but the place the field
is written.

### 16.3 Counting every block would have overstated the cache fourfold

The third is the one that would have produced a *wrong number* rather than no
number, which makes it the worst of the three. The configuration lists its layer
types:

| | |
|---|---|
| Blocks | 64 |
| Of which full attention | **16** |
| Of which linear attention | 48 |
| Key/value heads | 4 |
| Head dimension | 256 |

Only the full-attention blocks hold a key/value cache. Counting all 64 gives 256
KiB per token against the true 64 KiB — 1 GiB of cache at 4096 tokens where the
real figure is 256 MiB. On a 16 GB accelerator that is the difference between a
variant fitting and not.

MCF now counts the blocks the configuration says cache, and refuses to plan for
a configuration that lists layer types of which *none* caches, because that is a
shape MCF does not understand rather than a model that costs nothing.

### 16.4 What the plan says, now that it can be made

All 33 files classified, no bytes fetched. At 4096 tokens on 59.5 GB of usable
host memory every variant fits, which is the honest answer for **host** memory
and not the interesting one: the machine's accelerator holds 16 GB, and the
figures MCF computes make the comparison possible for whoever is choosing.

| Variant | File | Needs at 4096 tokens |
|---|---|---|
| `UD-IQ4_XS` | 14.25 GB | 15.06 GB |
| `UD-Q4_K_S` | 15.36 GB | 16.16 GB |
| `UD-Q4_K_M` | 16.46 GB | 17.27 GB |
| `Q8_0` | 29.05 GB | 29.85 GB |
| `BF16` (two parts) | 54.66 GB | — |

*Needs* is weights plus 256 MB of cache plus D24's stated 512 MB of runtime
overhead. Against 16 GB of accelerator memory, `UD-IQ4_XS` is the largest that
leaves room to work in; `UD-Q4_K_M` does not fit at all once the cache is
counted.

### 16.5 What this says about the method

Three defects, all in code with tests, all found by pointing it at one real
repository for the first time. Two of them produced *no answer*, which A7 made
safe; the third would have produced a **confident wrong one**, which is the
failure this project is organized against.

That is B-029's online tier earning its place — it acquires from the real hub on
a schedule — and an argument for extending it: the online check acquires a
1.2 MB model from a tiny-model repository, which exercises the transfer and not
the *variety* of what a hub publishes. A repository with a nested configuration,
a hybrid attention scheme and an exponent in its metadata is not an edge case;
it is the reference model.

## 17 · F17 — What a hub says when something is not there (DEC-038, §7.38)

**Why it was run.** MCF pins revisions and nothing says what happens when the
pin goes bad underneath it — a repository withdrawn, gated after acquisition,
relicensed, or a tag repointed (§7.38). What MCF is *able* to detect is a
property of the hub rather than of MCF, so it was asked.
`prototypes/upstream-decay/measure.sh`, anonymously, metadata only, no model
downloaded.

### 17.1 What it said

| Asked | Answer |
|---|---|
| A repository that does not exist | **401** |
| A gated repository, its card | 200, with `"gated":"manual"` |
| A gated repository, its file list | 200 |
| A gated repository, one file | 401 |
| A repository that is there | 200 |
| A revision that is not there | 404 |

The body of the first is `{"error":"Invalid username or password."}`.

**The hub does not distinguish *gone* from *private* from *never existed*, and
says so in the least helpful way available: by asking for a password.** That is
not an accident and it is not a defect — telling an anonymous caller that a
private repository exists is a leak — but it decides what MCF can honestly say.
A withdrawn pin and a typo produce the same answer, and no credential MCF could
be given would tell them apart unless the operator happens to have access to the
thing that is gone.

**A gate is visible before it bites.** The model card is readable and carries
`"gated":"manual"`; the file is not. So a repository that becomes gated *after*
acquisition is detectable — the flag changes, the card still answers — which is
the one decay in §7.38's list that MCF can name precisely.

**404 means one thing only: a revision that is not in a repository that is.**
Which makes it the reliable signal for a repointed or withdrawn *revision*, as
distinct from a repository.

### 17.2 The two decays that never refuse anything

A relicensing and a repointed tag are both a field with a different value and a
200 beside it. Nothing fails; nothing is refused; the request an operator makes
succeeds. They are detectable only by MCF **looking and comparing against what
it recorded at acquisition** — which is the same shape B-058 already has for
declared-versus-verified, applied along time instead of across sources.

### 17.3 What this settles

It gives DEC-038 the half that had to be measured, and the answer constrains the
decision rather than confirming it:

- **MCF can detect** a repointed or withdrawn revision (404), a gate that closed
  (the flag), a relicensing (the field), and any change of digest or size in the
  listing.
- **MCF cannot detect** the difference between withdrawn, private and
  never-existed. Anything it says about a 401 must carry that ambiguity, which is
  D33's shape exactly: report what was observed, and name the question being left
  unanswered.

MCF's current refusal on a 401 says *supply a credential*, which is right for a
private repository and misleading for a withdrawn one — advice that cannot work,
offered as though it could. That is corrected where it is written rather than
here.

## 18 · F18 — The wall three findings kept hitting is a packaged toolchain (B-320, B-183, DEC-047)

**Why it was asked.** Three separate findings have run into the same obstacle
and each treated it as fixed. F12 could not build either engine candidate for
`x86_64-unknown-linux-musl` — llama.cpp because it is C++, candle because
`tokenizers` reaches Oniguruma — and concluded that the musl question does not
separate them. F14 found SQLite's amalgamation unbuildable for the same target.
F15 recorded, in passing, that no C cross compiler is on this machine. Three
conclusions rest on an absence that nobody had priced.

**What was run.** `dnf info` and `dnf search`, which query this machine's
package manager and install nothing.

| Package | Download | Installed | What it is |
|---|---|---|---|
| `musl-gcc` | 12.0 KiB | 7.2 KiB | a wrapper that points the host gcc at musl |
| `musl-devel` | 214.6 KiB | 590.2 KiB | headers |
| `musl-libc-static` | 1.7 MiB | 8.0 MiB | the static library |
| `musl-clang` | 12.1 KiB | 7.8 KiB | the same wrapper for clang |

All from Fedora's own repository, from one source RPM, at musl 1.2.5.

**What this establishes and what it very deliberately does not.** It establishes
that a C toolchain targeting musl is **packaged for this machine** and costs
about nine megabytes installed. It does **not** establish that installing it
makes any of the three candidates build: a wrapper that compiles C for musl says
nothing about a C++ project's CMake, about `cc-rs` finding the right linker for
a Rust build script, or about Oniguruma's own configure. Each of those is a
thing to try, and trying it means changing a machine somebody else is using —
which is a decision for its operator rather than for a measurement.

**Why it matters anyway.** F12 left B-320 open partly because *neither candidate
keeps B-183's check runnable on a machine without a C cross toolchain*. That
sentence is true and its weight is different now: the machine can have one for
the price of a small package, so the question becomes *are we willing to require
a toolchain at build time* rather than *is it possible at all*. B36 constrains
what a **user** needs (nothing), not what a **build** needs, and MCF already
requires a pinned Rust toolchain nobody calls a violation.

The counter-argument survives and is worth stating: every build-time requirement
is a thing that breaks on somebody else's machine, and D29's per-platform
artifacts multiply it by the number of platforms. That is an argument about
which engine to admit rather than about whether the wall is real, and B-320 is
where it gets made.

## 19 · F19 — Two defects a real model found in an hour, and the tests that could not (B-364, B-365, A19, D38)

**Why it was run.** The operator's machine holds 107 GB of models acquired
through another tool, and three of them declare the one architecture MCF's
engine implements. Running one is the cheapest possible test of everything
B-364 had just built, and the strongest evidence available short of an oracle:
either the text is coherent or it is not.

**What was run.** `mcf run` against `Mistral-7B-Instruct-v0.3`, 291 tensors,
225 of them `Q4_0` and one `Q6_K`, prompt *The capital of France is*.

**What came back the first time.** `/******/obiubreérceronulusříoid`, from a
prompt MCF said was 26 tokens long. Both halves of that were defects.

### 19.1 The tokenizer used an algorithm no model is tokenized by

MCF segmented text with a Viterbi search over the whole string, maximizing the
sum of the vocabulary's scores. That is SentencePiece's *unigram* algorithm and
it is not what these files are read with. Two things were wrong with it here:

- **The scores are not log-probabilities.** In this vocabulary, `▁The` scores
  −156, `▁capital` −5306, `T` −28479, and every byte-fallback token scores
  **0.0**. They are ranks: for every token, score = −(identifier − 1027). A
  search maximizing the sum therefore preferred byte tokens to every real word,
  which is exactly what the 26 tokens were.
- **The algorithm is a merge, not a search.** Local runtimes tokenize these
  files by seeding a chain with single characters, then repeatedly merging the
  adjacent pair whose combined spelling scores highest, ties to the left. MCF
  now does the same. Where SentencePiece's own Viterbi would disagree with that
  merge is a question for B-368's oracle rather than an assumption here.

Byte fallback also emitted *one* byte of a multi-byte character rather than all
of them, so a space that fell back arrived as a fragment and came back as a
replacement mark. It emits every byte now, and decoding gathers consecutive byte
tokens before reading them as UTF-8.

**Fixed, the prompt is six tokens: `<s> ▁The ▁capital ▁of ▁France ▁is`.**

### 19.2 `Q6_K` decoded every value correctly and put them in the wrong places

With the tokenizer right, the model still produced noise. The single `Q6_K`
tensor in that file is `output.weight` — the projection to logits — so a fault
there turns a correct forward pass into random-looking tokens, which is what it
did.

The format writes the four values a byte-pair produces at **strides of 32**:

```c
y[l +  0] = d * sc[is + 0] * q1;
y[l + 32] = d * sc[is + 2] * q2;
y[l + 64] = d * sc[is + 4] * q3;
y[l + 96] = d * sc[is + 6] * q4;
```

MCF wrote them consecutively. Every value was right and every value was in the
wrong place. `Q2_K` and `Q3_K` had the same class of error in their sub-block
walk — four shifts of the same thirty-two bytes, two runs of sixteen per shift —
and were rewritten against the source at the same time.

**Fixed, the same prompt answers `Paris, but the largest city is Marseille`**,
and *Write one sentence about rain* answers *Rain is a natural phenomenon that
occurs when water droplets fall from the clouds to the earth's*.

### 19.3 The tests could not have caught either, and that is the finding

Both decoders had tests. Both tests passed throughout.

The `Q6_K` test built a block where **every value was the same** and asserted
they were all there. They were — in the wrong order, which a list of identical
values cannot show. A test that cannot distinguish a permutation from the
identity is not testing placement, and placement was the whole of the defect.
The replacement gives every position a distinct value and a distinct scale, and
fails on the old code.

The tokenizer's tests were worse in a subtler way: the laboratory's fixture
vocabulary held **only whole words** — `▁yes`, `▁no`, `▁maybe`. No real
vocabulary looks like that, because every token in one was *built* by merging,
so the intermediate pieces are always present. The fixture could not be
tokenized by the correct algorithm at all, and so it could only be tokenized by
the wrong one. Both fixtures now carry the full ladder from `▁` and single
letters up to each word.

**The general lesson, stated so it can be applied to the next one.** A fixture
MCF writes to test MCF is a fixture shaped by the same misunderstanding as the
code. D26 already says a scenario should build the observable rather than the
cause; this is its sibling for fixtures — *a fixture that cannot be wrong the
way a real file is wrong hides the defect it was built to catch*. Real artifacts
are how that is escaped, and B-368's oracle is how it is escaped systematically.

## 20 · F20 — A second architecture, and two silences that fail differently (B-365, A2, A19, D26)

**What was run.** Qwen3 0.6B, as Ollama already holds it on this machine —
`sha256-7f4030…e1fa`, 28 blocks, 1024 wide, 16 query heads over 8 key/value
heads, a stated head width of 128 that is *not* the embedding divided by the
heads, and a `gpt2` byte-pair vocabulary of 151,936 tokens with 151,387 merges.
The prompt was `The capital of France is`, greedy, seed 0, twelve tokens, run
through `mcf run` on the build in the working tree.

**What it produced, in four states.** The engine was run with each of the two
corrections this finding is about present and absent, and the four outputs are
the finding:

| rotary pairing | per-head query/key norm | what came out |
|---|---|---|
| adjacent pairs (llama's) | not applied | `了吗了吗了吗了吗了吗了吗…` |
| split halves (Qwen's) | not applied | `了吗了吗了吗了吗了吗了吗…` |
| adjacent pairs (llama's) | applied | `the the capital of of the the country of in which the` |
| split halves (Qwen's) | applied | `Paris, and the capital of the United States is Washington,` |

**The first thing this establishes is that the two defects fail differently, and
only one of them looks like a defect.** A missing per-head normalization
collapses the model onto a single token and is unmistakable. A wrong rotary
pairing produces *English* — words in the right proportions, function words in
plausible places, and no answer. Anybody watching the third row without the
fourth beside it would report a small model doing what small models do. The
rotation was already suspected and already documented as the thing to suspect;
what the experiment adds is that its signature is fluency, so *fluent output is
not evidence the rotation is right*. Nothing short of a second implementation or
a known answer distinguishes row three from a model that is simply small (A19).

**The second is that neither defect was a mistake in arithmetic.** Both were
silences.

The rotary pairing is not in the file. GGUF states the base frequency and the
head width and never states which two components of a head turn together, so
every engine that runs these files carries a table from architecture to pairing
under some name, and MCF's absence of one was a table with one entry that never
said so. It is now `rotation_for`, written out, defaulting to llama's, with the
families that want the other named — because being wrong there does not fail, it
produces text.

The per-head normalization was worse, because MCF had the code for it. The
weights are two tensors per block that llama does not carry, the engine asked
for them with `if let Ok(weights) = self.tensor(…)`, and the tensor map was
built from a manifest that did not list them. The lookup failed on every block
of every model, and an `if let Ok` cannot say so. That is A2's silent failure
inside an engine written under A2 — the guard was shaped like an option and was
in fact an error being discarded. Optional tensors are now loaded by asking the
file whether it carries them; a tensor that is there and unreadable is a refusal
rather than an absence, which is the difference between an optional part and a
skipped one.

**What this says about where to look next.** Both defects are of the same
family: something the architecture requires that the *file* does not say and the
*llama* path does not need. That family has more members — gemma3's sandwich
normalizations, llama4's expert routing, the reference model's sixteen
full-attention blocks among sixty-four — and none of them will announce
themselves. The reference implementation of B-368 is the systematic answer; a
real file per family, run and read, is what is available before it.

**What was not established.** Nothing here is a speed, and it cannot be (B65).
Twelve tokens from one prompt at one seed is not a measure of quality, and the
fourth row's claim about Washington is the model's own. That the engine now
produces coherent text for two architectures says that the paths it exercises
are right for these two files; it says nothing about the paths they do not
exercise — long contexts, other quantization schemes, the grouped-query ratios
these two happen not to have.

## 21 · F21 — Six families for a tenth of one model's bytes (B-369, D40, DEC-054, §3.12)

**What prompted it.** The operator observed that development against the
reference model is slow, that §XII's choice of it was made for its quality
rather than for its fitness as a development subject, and that the models
already on this machine should not be used — their state is unknown and they are
all large. F19 had reached for those models precisely because they were already
there, which is how an unexamined convenience becomes a condition of the work.

**What was acquired.** The smallest *trained* model of each family MCF covers or
means to cover, one distinct quantization apiece so that architecture coverage
and quantization coverage come from the same six files. Every one was fetched by
`mcf pull`, digest-verified, and recorded with its provenance.

| family | artifact | quantization | bytes |
|---|---|---|---|
| llama, unigram vocabulary | `Felladrin/gguf-Llama-160M-Chat-v1` | Q4_K | 121,295,296 |
| llama, byte-pair vocabulary | `bartowski/SmolLM2-135M-Instruct-GGUF` | Q8_0 | 144,811,360 |
| qwen3 | `unsloth/Qwen3-0.6B-GGUF` | Q4_K_M | 396,705,472 |
| gemma3 | `unsloth/gemma-3-270m-it-GGUF` | Q6_K | 282,975,264 |
| mixture-of-experts | `RichardErkhov/Isotonic_-_TinyMixtral-4x248M-MoE-gguf` | Q5_K_M | 500,878,816 |
| embedding | `leliuga/all-MiniLM-L6-v2-GGUF` | Q4_0 | 19,699,648 |

**1.4 GB for six families against 16.5 GB for one model.** The two that run
today answer a five-token prompt in 3.9 s (160M) and 15.9 s (0.6B) on this
machine, measured with `time` around `mcf run` at a ten-token budget. These are
*not* speeds in B65's sense and cannot become them — they are the stand-in's own
cost and are recorded here only as the quantity the decision was about: how long
it takes to find out you were wrong.

**The other four refuse, and what they say is the finding's second half.** Each
names exactly what it wanted:

- the byte-pair llama asks for a pre-tokenizer called `smollm`, which MCF has
  not implemented and will not substitute (A7);
- the embedding model carries a third tokenizer scheme, `bert`;
- the mixture-of-experts file has no `blk.0.ffn_gate.weight`, because its gate
  is per-expert;
- gemma3 is an architecture MCF has not been taught, and says which it has.

Four refusals, four different subsystems, four different categories, and not one
of them a crash or a guess. That is the surface working — but note what it also
is: **the refusals are the order of work**, derived from artifacts rather than
from a list somebody wrote down. B-365's remaining sequence is now read off six
files instead of being predicted.

**What this costs, stated because it is not free.** A 135M model cannot referee
the difference F20 turned on. That finding separated a correct engine from a
subtly wrong one because the correct one answered `Paris` and the wrong one
produced fluent English containing no answer; the judgement needed a model good
enough to be right. The corpus buys iteration speed by giving up the oracle that
F20 used, and the only thing that buys it back is a reference implementation to
compare against numerically — which is D39's opening and B-368's item. That is
why DEC-054 moves B-368 ahead of the remaining families rather than after them.

**What was not established.** That these six are the smallest such models, only
that they are the smallest found by asking the hub for each family's known small
releases; a smaller trained gemma3 or mixture-of-experts may exist. Nothing here
measures quality, and the two timings measure this machine and this stand-in.
Whether the four refusals are the *only* things those files need is unknown —
a refusal names the first thing missing, not every thing.

## 22 · F22 — Where a model becomes able to referee an engine (B-370, D40, DEC-054, A19)

**The question.** F21 asserted that a small model cannot tell a correct engine
from a subtly wrong one, and used that to make the conformance corpus depend on
an oracle. The assertion was reasoning, not measurement. This measures it.

**The method.** Two corpus models were run against a correct engine and against
one with a single defect: the rotary pairing swapped, so each family gets the
other convention. That is the defect from F20 that produces *English* rather
than gibberish — the hard case, deliberately. Four prompts with answers a person
knows, greedy, seed 0, fourteen tokens. The engine was rebuilt from a
git-clean tree before each block; a first attempt at this measurement compared
two runs of the same stale binary and produced two identical tables, which is
how the rebuild came to be part of the method.

**Llama-160M-Chat, Q4_K:**

| prompt | correct engine | rotation swapped |
|---|---|---|
| The capital of France is | `Paris.` | `the capital city of Paris.` |
| Water freezes at a temperature of | `100 °C. The water freezes at a temperature` | `120°C. The water dropleases are formed` |
| The largest planet in our solar system is | `Mercury. Mercury is the second-largest planet in our` | `the Sun.` |
| The opposite of hot is | `hot.` | `hot.` |

**Qwen3-0.6B, Q4_K_M:**

| prompt | correct engine | rotation swapped |
|---|---|---|
| The capital of France is | `Paris. The capital of France is also the capital of the Republic of` | `the the capital of of the the country which is the the capital of` |
| Water freezes at a temperature of | `0°C, and the boiling point of water is at 1` | `at 200000000000` |
| The largest planet in our solar system is | `...? A. Mercury B. Venus C. Earth D. Mars` | `called the...? A.. B B.. C..` |
| The opposite of hot is | `cold, and the opposite of cold is hot. So, the opposite` | `a the of the the same as the opposite of cold. So,` |

**At 160M the reader cannot tell which column is the broken engine, and the
reason is not subtlety.** It is that the correct engine's own answers are wrong:
water freezing at 100 °C, Mercury as the largest planet, the opposite of hot
being hot. Three of four prompts are answered incorrectly by a *correctly
implemented* engine. A model that does not know the answer cannot be asked
whether the engine found it, and on one prompt the two engines produce
character-identical output. The one row that looks like a signal — `dropleases`,
which is not a word — is the only one, and one non-word in four prompts is not
something to build a tier on.

**At 0.6B all four are unmistakable, and the correct column is right.** Freezing
at 0 °C, the opposite of hot being cold. The broken column repeats function
words and emits a run of twelve digits. No judgement is required to separate
them.

**So the threshold is between the two, and it is cheap.** 0.6B is 397 MB and
answers in about sixteen seconds on this machine — not the 16.5 GB the reference
model costs. The useful statement is not *small models are unfit* but **a model
must be good enough to be right about the thing being asked**, and that turns
out to start well below a billion parameters.

**This corrects F21 and the decision that cited it.** F21 concluded the corpus
*depends on* an oracle because coherence cannot referee at small sizes. The
dependency is real at the bottom of the corpus and absent one step up. The
oracle is still worth building — it is exact where this is a judgement, it works
on the 160M model where this does not, and it catches defects that leave output
fluent at any size — but it is not a precondition for the corpus to be useful.
B-368 keeps its priority on its own merits and not on this one.

**What was not established.** One defect, one seed, one greedy sampler, four
prompts, two models. That 0.6B suffices *for this defect* is not that it
suffices for every defect; a defect subtler than a swapped rotation may be
invisible at 0.6B and visible only to a numeric comparison. The threshold is a
lower bound on what is needed, never an upper bound on what is enough (A21).

## 23 · F23 — Four expressions where MCF had two, and what no test could have found (B-365, B-370, A7, A19, A21)

**What prompted it.** SmolLM2 refused with `asked for: smollm`, and MCF's
pre-tokenizer table had to grow by one. Rather than infer what `smollm` means
from its name, the reference implementation was read — llama.cpp's
`llama-vocab.cpp`, the `switch` on the pre-tokenizer type and the table that
maps the string in `tokenizer.ggml.pre` onto it. That reading found three
things wrong with what MCF had already shipped.

**One: `qwen2` and `llama-bpe` are not the same expression.** MCF had them as
one, on the strength of how similar they look. They differ in one place:
llama3 takes digits in groups of up to three, `\p{N}{1,3}`, and qwen2 takes them
one at a time, `\p{N}`. A different cut is a different set of merges that can
apply, so a number tokenizes differently under the two.

**Two: `deepseek-llm` was claimed and is not implemented.** MCF listed it in the
same group. Its pre-tokenizer is six expressions, one of which is an explicit
enumeration of several hundred letter ranges. It is now refused by name.

**Three: `default` is not GPT-2's expression.** MCF read a file with no
`tokenizer.ggml.pre` as GPT-2, reasoning that the field postdates the format.
llama.cpp's fallback for that case is a *fourth* pattern — four expressions,
one of which splits on punctuation. A file that does not say is now refused
rather than run through something that resembles what it wants.

**None of the three could have been found by running anything, and the first
one demonstrates why.** Qwen3-0.6B tokenizes `What is 1234 plus 5678` into
thirteen tokens with every digit separate — *both before and after* the fix.
Its merge list contains no merge that joins two digits, so the grouping never
had anything to group and the two cuts agree on this artifact. The defect is
real, it is fixed, and no prompt against this model distinguishes the fixed
version from the broken one. What a name maps to is a fact about somebody
else's software; the only instrument that reads it is somebody reading it
(A21: declared is not verified — and here, *not verifiable by observation*).

**What the corpus did catch.** With `smollm` implemented, SmolLM2 runs — and
the tier reported it as a failure, which is what it is for: the register said
that entry refuses, it no longer does, and good news nobody notices is how a
check stops being read.

**And SmolLM2 corrects F22.** That finding put the floor for refereeing an
engine "between 160M and 0.6B". SmolLM2-135M-Instruct at Q8_0 is *smaller* than
Llama-160M-Chat at Q4_K and answers all three probe questions correctly —
`Paris.`, `0 degrees Celsius.`, `cold.` — where the 160M model got every one of
them wrong. Run with the rotary pairing swapped it gives `Paris.`,
`10 degrees Celsius.`, `10.`: two of three visibly degraded.

So the floor is not a parameter count. **It is whether that artifact, at that
quantization, knows the answer to what is being asked** — and a well-trained
135M model at Q8_0 knows more of it than a poorly-trained 160M model at Q4_K.
F22's measurement stands; its generalization to a size does not.

**A fourth defect, found by the round-trip and not by reading.** Every corpus
vocabulary is now encoded and decoded over awkward text — digit runs,
punctuation against letters, characters that are several bytes, runs of
whitespace, the empty string. The empty string came back wrong for every
unigram vocabulary: adding control-token matching to `encode` had introduced a
walk that skipped an empty remainder, silently turning *encode nothing* from
one token into no tokens. It has a test now.

The round-trip also showed something that is **not** a defect and had to be
told apart from one: `SentencePiece` puts a space in front of every text, so
`decode(encode(t))` is `" " + t` and no decoder can tell that space from one
the text really had. MCF now reads `tokenizer.ggml.add_space_prefix` where a
file states it, and the check allows the convention for exactly the
vocabularies that declare it rather than stripping a leading space and eating a
real one.

**What was not established.** That the four expressions are now right — only
that they are transcribed from a reference that runs these models, and that
three vocabularies round-trip. The transcription itself is unverified in the
sense A21 means, and B-368's oracle is what would verify it: two
implementations agreeing on identifiers for the same text is a check, and one
implementation agreeing with itself is not.

## 24 · F24 — A third architecture, and three habits that fail three different ways (B-365, B-370, A19, §3.18)

**What was added.** Gemma 3, read from llama.cpp's `models/gemma3.cpp` rather
than inferred, and run against the corpus artifact `gemma-3-270m-it` at Q6_K —
18 blocks, 640 wide, 4 query heads over 1 key/value head, a stated head width
of 256, and a unigram vocabulary of 262,144 tokens that declares
`add_space_prefix = false`.

It differs from llama in five places. Two the file states and three it does not,
and that division is the finding.

**What the file states, and is therefore read from the file (§3.18):** two more
normalizations per block — on the way *out* of the attention half and out of
the feed-forward half, which is what makes these blocks a "sandwich" — carried
as `post_attention_norm.weight` and `post_ffw_norm.weight`. MCF applies them
because the tensors are there, so a file of any family carrying them gets them
and a gemma file without them would not.

**What the file does not state, and is therefore a table:** the gated block
activates with `GELU` rather than `SiLU`, the embedding is multiplied by the
square root of its width on the way in, and the rotation pairs halves rather
than adjacent components. None of the three is visible in any tensor or any
metadata key. A `GELU`-gated block and a `SiLU`-gated one have the same tensors
of the same shapes.

**Each was removed in turn, and they fail in three different registers.** The
prompts are `The capital of France is` and `The opposite of hot is`; correct,
the model says `Paris.` and `cold.`

| what was removed | what came out |
|---|---|
| the two sandwich normalizations | `incessant intensive intensiveHighwayHighwayHighway…` |
| `GELU`, using `SiLU` instead | `the 12th city, and the` · `a cold. It's a cold, a` |
| the embedding's scale | `चौ चौ चौلسللسللسللسل…` |

Word salad, fluent-and-wrong, and characters from another script. **Only the
middle one is dangerous**, and it is the one that would survive review: real
words, plausible grammar, and no answer. It is the same signature F20 measured
for the rotary pairing and F22 measured again — and it is now three habits, in
two different families, that produce it. That is no longer a coincidence worth
noting once. *An unobservable habit, got wrong, produces fluent text* looks like
the general case, and the loud failures are the lucky ones.

**Where this leaves the division of labour.** Everything observable is read
from the artifact, which is why gemma3 needed no new code path for its
normalizations — only two more names in the list of tensors MCF will load if
they are there. Everything unobservable is in one table, `architecture.rs`,
which DEC-053 already requires and which the neutrality check already holds to
naming no artifact. Growing a family is now: read the reference, add what the
file cannot tell you, and let the rest be found.

**What was not established.** Gemma 3 alternates local and global attention on a
five-to-one pattern with a different rotary base for each, and this artifact
declares a sliding window of 512 but **no** separate base for the local layers.
MCF therefore uses the one base the file states, for every layer, which is what
the file says and not necessarily what the model was trained with (A7, A21). At
twelve tokens no window truncates anything, so nothing here exercises the
sliding window at all. A file that declares a second base, or a prompt longer
than 512 tokens, is untested ground.

Nor is the 27B variant's attention scale exercised: gemma scales queries by the
inverse root of the head width except at 27B, where it is the inverse root of
the embedding over the heads. On this artifact those coincide with what MCF
already computes.

## 25 · F25 — The broken engine read better than the correct one (B-365, B-368, A19, D38)

**What was added.** A mixture of experts, read from llama.cpp's
`build_moe_ffn`, and run against the corpus artifact `TinyMixtral-4x248M-MoE` at
Q5_K_M — twelve blocks, four experts per block, two used per token.

Notably the file declares `general.architecture = llama`. Whether a block holds
one feed-forward or a stack of them is not a property of the family: it is
`llama.expert_count`, which the file states, so MCF reads the shape from the
count and not from the name (§3.18). Nothing about "mixtral" appears anywhere in
the engine.

**What a mixture adds is a choice.** Each expert is an ordinary feed-forward run
with the same three matrix multiplies. The new part is: score every expert
through a small router, softmax the scores, take the highest two, renormalize
those two so they sum to one, and add their outputs in proportion.

**Two pieces of that were removed in turn, and here is the result that matters.**
The prompts are `The capital of France is` and `The opposite of hot is`.

| version | what came out |
|---|---|
| as written | `Paris, France is Paris, Paris is Paris is` · `hot. The hot is hot, the hot is` |
| the router ignored — experts 0 and 1 every time | `Paris. It is located on the River Seine in` · `hot.` |
| the two weights not renormalized | `France France France France France France` · `hot` |

**The broken version reads better than the correct one.** Ignoring the router
entirely — never scoring an expert, always taking the first two — produces
`Paris. It is located on the River Seine in`, which is a fluent, accurate,
well-formed sentence. The implementation transcribed from the reference produces
a stammer. Anybody tuning this by reading the output would have deleted the
router and called it an improvement.

That is the same phenomenon as F20's rotation, F22's measurement and F24's three
habits, and it is now at its limit: it is not merely that a wrong engine can
look right, it is that **a wrong engine can look better than a right one**, on
the same artifact and the same prompt. Output quality is not evidence about
implementation correctness in either direction. This finding is the case for
B-368 and there is no longer a stronger one to be made.

**What is actually known about this implementation.** That it was transcribed
from the reference rather than inferred; that both the routing and the
renormalization are live, because removing either changes the output; that the
shape is read from the file's own counts and refuses a file that states experts
without stating how many are used. That is not the same as knowing it is right,
and the difference is exactly what an oracle would close (A21).

**Why the artifact is a poor witness, stated because it bears on the above.**
`TinyMixtral-4x248M-MoE` is a merge of four small models rather than a mixture
trained as one, so its own quality is low and its stammer is a plausible thing
for it to do unassisted. A better mixture would make the correct column read
better — and would not change the argument, because the broken column would
still have read fluently.

**What was not established.** The weight scale some mixtures apply
(`expert_weights_scale`), which this file does not declare and MCF therefore
does not apply; expert biases and grouped routing, which belong to other
families; and whether two of four is representative of routing at a realistic
width. Nothing here is a speed (B65).

## 26 · F26 — The oracle found a defect on its first run, and it was an invention of MCF's own (B-368, A19, A21, §3.12)

**What was built.** llama.cpp at commit `925e1179`, built from source as a
development instrument — vendored nowhere, on no MCF command's path, and absent
from every artifact MCF ships (D39's fourth condition). The build queued behind
another project's run on the shared machine rather than taking the window from
it.

**What is compared, and why only this.** Token identifiers. They are integers:
two tokenizers either agree about a list of them or do not, with no tolerance to
argue about and no floating-point arithmetic in the way. Logits are deliberately
not compared yet — a correct implementation can flip an argmax on a near-tie
through nothing worse than a different summation order, so a disagreement there
is a finding to investigate rather than a verdict.

Six texts across five corpus vocabularies: a plain sentence, digit runs,
punctuation against letters with contractions, characters outside ASCII, a tab
and a double space, and a single letter.

**The result: 29 of 30 agreed exactly, and the thirtieth was a defect.**

```
gemma-3-270m-it   on  "tabs\tand  spaces"
    MCF:        2 39218 255968 624 236743 9952
    reference:  2 39218 255968 624 138 35220
```

The two agree for four tokens and part at the double space. Token 138 in that
vocabulary is `'  '` — two literal spaces, marked **user-defined**. MCF was
segmenting it into `▁` and `▁spaces`; the reference matches it whole.

**The cause was a guard MCF invented.** MCF matches user-defined tokens in the
raw text before segmenting, and the code that did it skipped any token shorter
than three bytes, on the reasoning that *a one-character token would match
inside ordinary words*. That reasoning is plausible and is not what any
implementation does. The reference matches every user-defined token whatever its
length, sorted longest-first so a longer token claims its text before a shorter
one can — the sorting MCF already did, with a guard on top that nobody else has.

Gemma's vocabulary carries 163 user-defined tokens, many of them runs of
whitespace. Any text containing a double space tokenized differently in MCF than
in every other implementation of the same file.

**A second thing the reference settled, which MCF had guessed at.** MCF matched
*control* tokens in ordinary text too — `<|im_start|>` typed into a prompt became
the token a chat template uses to start a turn. The reference's default is to
leave control tokens as text and match only user-defined ones, and that is both
the compatible answer and the safer one: a prompt is text somebody typed, and it
should not be able to produce a marker by spelling it. MCF now does the same, and
a surface that applies a chat template will have to ask for the other behaviour
rather than receive it by accident.

**Why this finding matters more than the defect in it.** The defect is small.
What it demonstrates is the thing four previous findings argued for and could
not supply: F20, F22, F24 and F25 each established that output is no evidence
about correctness, and each ended by saying an oracle would settle it. This is
the first check MCF has that can say *wrong* about an engine without a human
judging a sentence — and the first time it ran, on five models it had been
producing plausible text with all day, it found something.

Note also what found it: not the plain sentence, and not the model failing to
say `Paris`. Gemma answers `Paris.` and `cold.` correctly with the defect
present. It was the tab-and-double-space string, which exists in that list
precisely because whitespace is where tokenizers differ.

**What is now known that was not.** MCF's four pre-tokenizer expressions, its
byte-level alphabet, its merge ordering and tie-breaking, its unigram
segmentation, its byte fallback, its space prefix and its beginning-of-text
handling agree with the reference on thirty comparisons across five vocabularies
of two schemes. F23 recorded three transcription defects found by reading and
said plainly that the transcription itself was unverified. That part is now
verified — for these vocabularies and these texts, which is what a check
establishes and not more (A21).

**What was not established.** Nothing about the forward pass: the arithmetic
that F20, F24 and F25 were about is untouched by this, and remains transcribed
and unchecked. Nothing about the embedding family, whose vocabulary MCF still
refuses and which was reported as not compared rather than counted as agreeing
(A4). And nothing about texts unlike these six.

## 27 · F27 — What a coin-flip looks like, and what a defect looks like (B-368, B-365, A19, A21)

**The question this had to answer before the forward pass could be compared at
all.** Greedy generation is deterministic, so two correct implementations should
produce the same tokens from the same model and prompt. Except they need not:
where the best and second-best logits are close enough, a different order of
summation picks a different winner, and neither implementation is wrong. F26
compared identifiers precisely because that comparison has no such problem —
integers agree or they do not. Comparing generated text needs a way to tell a
coin-flip from a defect, and MCF had asserted one was needed without measuring
it.

**The instrument.** `margins`, which reports for every step of a generation the
gap between the chosen token's logit and the runner-up's. The claim it makes
possible is: *a divergence at a small margin is arithmetic; a divergence with
room to spare is a defect.*

**Fifteen comparisons, five models, three prompts, ten tokens each.** Eleven
agreed exactly, token for token. Four diverged, and every one of them looks the
same:

| model · prompt | margin where they parted | MCF chose | reference chose |
|---|---|---|---|
| SmolLM2 · *Water freezes…* | **0.040** | ` freezing` | ` equation` |
| Llama-160M · *Water freezes…* | **0.098** | ` water` | ` free` |
| SmolLM2 · *The capital of France is* | **0.105** | ` the` | ` a` |
| gemma3 · *Water freezes…* | **0.159** | `2` | `0` |

In **every** case the reference's choice was exactly MCF's runner-up, and in
every case the divergence happened at the smallest margin in that whole
generation. That is what a coin-flip looks like: the two implementations
disagree only where the model itself was indifferent.

**And then the defect, which looks nothing like it.** Before the above, gemma3
diverged on *The opposite of hot is* at a margin of **0.775**, choosing `Hot`
where the reference chose `The`. Five to nineteen times the largest noise
margin. That was real, and this is what it was.

Gemma 3 alternates sliding and global blocks — five that see only the last 512
positions, then one that sees everything — and the two kinds **rotate at
different base frequencies**. The corpus artifact declares
`gemma3.attention.sliding_window = 512` and does *not* declare a base for the
sliding blocks. F24 recorded exactly this and called it untested ground: MCF
used the one base the file states, for every block.

What the reference does when that key is absent is use **ten thousand** — a
default in its own header, not anything the file says. So MCF was rotating five
blocks in six at 1,000,000 where every other implementation rotates them at
10,000. The model still answered `Paris.` and `cold.` correctly, which is why
nothing before this caught it.

MCF now reads `rope.freq_base_swa` where a file states it, falls back to ten
thousand where it does not, and applies the window itself — a key at position
`p0` is visible from `p1` only while `p1 - p0` is under the window, which is the
reference's boundary.

**The tolerance, and its honest width.** The check now fails a generation
divergence only when *every* step of it had a margin over **0.50** — above the
largest noise observed by a factor of three, below the one observed defect by a
factor of one and a half. Reintroducing the sliding-window defect makes it fire:
`closest margin 0.95879, over 0.50 — this is not a near-tie`, and the run exits
non-zero after printing its summary.

The threshold is provisional and the reason is worth stating plainly: **a defect
can hide under a near-tie.** If a wrong engine happens to be wrong only where the
model was indifferent, this passes it. What narrows the gap is more comparisons
and more prompts, not a cleverer rule; four noise observations is what this rests
on.

**What is now known.** Across five corpus models, MCF's tokenizer agrees with the
reference on thirty comparisons exactly, and its forward pass agrees on eleven of
fifteen generations token for token, the other four differing only where the
model was indifferent. F20, F24 and F25 each ended by saying the forward pass
was transcribed and unchecked. It is no longer unchecked — for these models,
these prompts, and ten tokens, which is what a check establishes and not more.

**What was not established.** Nothing about long contexts: at ten tokens the
sliding window never truncates anything, so the *mask* MCF now applies is
exercised by nothing here and only the rotary base was measured. Nothing about
the embedding family, still refused. Nothing about sampling other than greedy.
And nothing here is a speed (B65).

## 28 · F28 — The mask, past the boundary (B-368, F27, A21)

**The hole this closes.** F27 fixed gemma3's sliding-window rotary base and
*implemented* the window's mask, then said plainly that at ten tokens the mask
is exercised by nothing. A defect in a boundary nobody crosses is invisible, and
the mask MCF now applies had never once masked anything.

**Two prompts, both past the boundary, chosen for different failure shapes.**
Gemma3's window is 512 positions; five blocks in six see only that far back.

- **682 tokens of one sentence repeated**, then eight generated. The blandest
  possible history: if the mask boundary were off by one, the sliding blocks
  would attend to a slightly different set of near-identical keys — a defect
  with room to hide. Both implementations produce `The quick brown fox jumps
  over the`, token for token.
- **A distinctive fact, then 550 tokens of filler, then a cue** — `My name is
  Konstantin Aurelio Blackwood… [filler] …My name is`. The name sits *outside*
  every sliding block's window and inside the global blocks' view, so the two
  kinds of block must disagree about what the history holds, and only an engine
  that masks the sliding ones and not the global ones recalls it. Both
  implementations produce `Konstantin Aurelio Blackwood. I live`, token for
  token.

Both tokenizations were verified identical before comparing (682 and 621
identifiers), so the generations compare the forward pass and nothing else.

**What this establishes.** MCF's window mask — a key at `p0` visible from `p1`
only while `p1 − p0` is under the window — agrees with the reference's at 682
positions, on a boundary that is actually crossed, in both a history where the
masked keys resemble the kept ones and a history where they do not. The
comparison is available on demand as `MCF_ORACLE_LONG=1 scripts/check-oracle.sh`
and is off by default: the stand-in pays one forward pass per prompt token, and
a tier that costs minutes by default stops being run.

**What was not established.** One window size, one pattern (five sliding to one
global), one model. A file whose window differs from 512 or whose pattern the
file states explicitly exercises arithmetic this did not. Nothing here is a
speed (B65).

## 29 · F29 — The sixth family answers a different question (B-371, DEC-055, B-365, A19, §3.3)

**What was built.** The bert family: a WordPiece tokenizer, a whole-sequence
non-causal forward pass with classic layer normalization and biases throughout,
mean pooling as the file declares, and a new surface — `mcf embed <model>
--text <text>` — because the question these models answer is not `mcf run`'s
question. There is no output head and no next token; there is one vector for
the text, and DEC-055 is resolved by giving that its own verb. The vector goes
first and machine-readable — one JSON line, `{"width":…,"embedding":[…]}` —
and the conditions after, legible: token count, the declared pooling, the unit
normalization, and the same B65 mark every stand-in answer carries.

**Everything transcribed was checked against the reference the same day it was
written**, which is what B-368 existing before B-371 was for.

*The tokenizer, exactly.* Five texts — plain English, punctuation with
contractions, `café naïve 你好`, digits, and invented words that must become
`[UNK]` — agree with the reference identifier for identifier. The normalizer's
scope is stated in the module rather than hidden: accents are stripped by a
Latin fold table because MCF carries no Unicode tables, and a precomposed
accented letter outside Latin passes through where the reference would
decompose it (A21).

*The forward pass, at a measured distance.* MCF dequantizes to floats and
multiplies; the reference multiplies in quantized arithmetic and quantizes the
activations too. The vectors therefore cannot be equal, and how unequal is the
measurement: across five texts, cosine 0.999596 to 0.999811, largest single
component difference 0.006. The oracle's floor is set at 0.999 — under the
observed agreement, and far above what anything structural leaves standing:
swapping one normalization in one layer from LayerNorm to RMS drops cosine to
0.972–0.988, and the tier fires on every text.

*That the vector means something*, which agreement alone does not show: `The
cat sat on the mat` against `A kitten rested on the rug` scores 0.62; each
against `Quarterly revenue grew by twelve percent` scores under 0.03.

**The corpus is six for six.** Every family acquired in F21 now either runs or
embeds through MCF's own engine, and each got there the same way: read the
reference, take from the file everything the file states, put what no file
states where DEC-053 requires, and let the oracle say whether the transcription
is right. Four defects were found on that road (F23 ×3, F26, F27) and none by
reading output.

**What was not established.** One bert model, quantized one way, on short
English-heavy texts. The nomic-bert variant on this machine adds rotary
positions and a gated feed-forward and is not covered by anything here. The
first-position pooling variant is implemented and exercised by no artifact. And
cosine at 0.999 is a floor calibrated on this model at this size — a larger
model's arithmetic gap may sit elsewhere, and the floor would need remeasuring
rather than trusting (A21). Nothing here is a speed (B65).

## 30 · F30 — Two ways to provision the same component, measured (DEC-052, B-367, D39, A27)

**The question.** D39 admits components MCF provisions — installs, builds and
pins itself — and names four conditions: pinned and recorded, reproducible,
contained and reversible, never the baseline. DEC-052 asks what mechanism makes
"controlled" true. Two candidates existed on paper: a managed prefix built with
the host's own tools, and a container. Both have now provisioned the same
component — llama.cpp at commit `925e1179`, the oracle — and the differences
were measured rather than argued.

**Route A: the host's tools into a prefix.** This is how the oracle was first
built (F26): clone pinned, `cmake`, build into `/home/gauge/Content/mcf-oracle`.
It worked, and it failed D39's first condition in a way nobody would have
noticed: **the build used cmake 4.2.1 resolved from a pyenv shim** —
`~/.pyenv/shims/cmake` — while the system package, which is what anyone
recording the environment would have asked `rpm` about, is 4.3.0. The toolchain
that built the oracle was an accident of the operator's Python setup, recorded
nowhere, and different from what every reasonable record would have said it
was. That is "whatever the PATH resolved today", observed in the wild on the
first provisioning MCF ever did.

**Route B: a rootless container over a mounted prefix.** `podman run --rm` on
`fedora:44` (pinned by digest `5a4a491c…`), the source mounted read-only, a
prefix on the content drive mounted for output, the toolchain installed inside
and its exact versions written into the prefix: gcc 16.2.1-2.fc44,
cmake 4.3.0-1.fc44, glibc 2.43-8. The build ran under the shared machine's
exclusive window like any other heavy work.

**What was measured.**

| property | route A (host prefix) | route B (container) |
|---|---|---|
| toolchain | ambient — a pyenv cmake, discovered after the fact | enumerated — image digest + package versions, written into the prefix |
| residue outside the prefix | build used host state; nothing installed | **zero bytes**: container storage byte-identical before and after (`--rm` discards the layer the toolchain was installed into) |
| the artifact on the host | runs (it is a host binary) | **runs, and agrees**: the container-built `llama-tokenize` on the host produces identifier-for-identifier the same output as the host-built one |
| removal | `rm -rf` the prefix | `rm -rf` the prefix; the base image was already present and is shared |
| cost of a rebuild | incremental | the toolchain reinstalls every run, because the layer that held it was discarded |

**What decides it.** Route A's failure is not hypothetical — it happened, on
the first try, silently. Route B's costs are visible and payable: a rebuild
re-resolves packages unless the image is derived and kept, and a binary built
against a container's glibc runs on the host only while the container's glibc
is not newer than the host's — true here by construction, since the image is
the host's own distribution and release, and a boundary any implementation must
check rather than assume.

DEC-052 is therefore resolved: **a controlled environment is a container MCF
drives — rootless, base image pinned by digest, source mounted read-only, a
prefix the operator can point at any drive mounted for output, and the
component's exact package set recorded into the prefix beside what was built.**
The managed prefix survives as the *shape of the output* — everything lands in
one removable directory — and the container is what makes the inputs
enumerable.

**Afterwards, the reversibility claim was exercised rather than asserted:** the
experiment's prefix was removed with one command, and the container store had
nothing to remove.

**What was not established.** Bit-identical rebuilds were not measured — two
builds from the same image and commit may still differ by timestamps, and D39's
second condition asks for "the same environment, or say what differed", which
package-set recording satisfies and binary identity would exceed. GPU and
vendor-runtime provisioning — the case that motivated D39 — adds device access
to the container and was not exercised. And `podman` itself is now a condition:
a machine without it falls back to nothing, because route A is what falling
back looks like.

## 31 · F31 — What the first automated provisioning found in two tries (B-367, DEC-052, A27, §3.15)

**What was built.** `mcf provision <component>`: F30's manual run turned into a
command. A table of components in MCF's source — one entry, the reference
implementation — each pinning an image by digest, a source by commit, a package
list, a configure line and targets. One `podman run --rm` over the pinned
image with a prefix bind-mounted; the recipe written into the prefix as a
script before it runs; the package versions, the landed commit and the log
written into the prefix by the run; `mcf-provenance.json` beside the build and
a `component_provisioned` entry in the record on success. Removal carries a
reason and is recorded before the directory goes.

**The first run failed in 30 seconds, and the failure was MCF's.** Exit 126:

```
Error: cannot chown /home/gauge/Content/mcf-data/containers/storage/overlay/…/merged
to 0:0: … read-only file system
```

Rootless podman keeps its *image store* under `$XDG_DATA_HOME/containers`.
MCF's data home on this machine is the large content drive — the operator's
instruction, and the right place for a prefix — and podman inherited it. That
drive's filesystem presents every file as root-owned and will not perform the
ownership changes an overlay store needs. 196 MB of image were pulled into a
store that could not hold them before the run stopped. F30's manual run had
not hit this only because it ran with the data home unset.

The division that holds: the *prefix* is MCF's and goes where the operator
says; the *store* is podman's, shared with everything else on the machine,
and goes where the platform puts it. `mcf provision` now hands its child the
platform default for the store and the operator's choice for the output. The
misplaced store was removed by hand, and that is the last time it will need
to be.

**The second run failed after the clone, and the failure was git's guard.**
Exit 128: `fatal: detected dubious ownership in repository at '/work/source'`.
The same filesystem, seen from inside the container, presents the freshly
cloned repository as owned by somebody else, and git refuses to operate in a
repository it thinks it does not own. The clone had succeeded and the package
versions were already recorded; the checkout was what refused. The exception
is now passed per invocation — `git -c safe.directory=/work/source` — scoped
to one directory for the life of one command, which is exactly as far as it
should reach.

**Both failures did what A2 asks.** Each named its exit status and its log,
said the prefix was safe to remove and the run safe to repeat, and left the
record untouched — nothing was written as provisioned that was not. What
neither could do was *say* which of the four D39 conditions was in play, and
that is worth noticing: the platform-mechanism category carried both with
distinct details, and a reader of the record would need the log to tell them
apart. Whether provisioning deserves categories of its own is a question for
when there is a second component.

**The third run succeeded, and found the fourth thing.** Image pulled by
digest, packages resolved and recorded exactly — gcc-c++ 16.2.1-2.fc44,
cmake 4.3.0-1.fc44, git 2.55.0-1.fc44, make 4.4.1-12.fc44, glibc 2.43-8.fc44 —
the checkout verified against the pin, three targets built, 656 MB in the
prefix, `mcf-provenance.json` beside the build, a `component_provisioned`
entry in the record, and the container store byte-identical to before (195,424
KB) with nothing created under MCF's data home. Run from the host against the
corpus's embedding model, the provisioned `llama-tokenize` produced identifier
for identifier what the hand-built one did.

*With an incantation.* Without `LD_LIBRARY_PATH` it loaded nothing:

```
error while loading shared libraries: libllama-common.so.0: cannot open shared object file
RUNPATH: [/work/build/bin:]
```

The build was shared, and a shared build bakes the library path *at build time*
into every binary — `/work/build/bin`, which is where the build directory was
inside the container and nowhere on the host. The hand-built oracle had never
shown this because its RUNPATH was a real host path. The artifact was correct,
complete, recorded, and unusable where it landed. The recipe now builds
self-contained (`-DBUILD_SHARED_LIBS=OFF`), the test that holds every recipe
requires it, and the shared build was removed *through MCF* — `mcf provision
--remove … --because "built shared: its RUNPATH names /work/build/bin …"` —
so the record says why a build that worked was thrown away (A27).

**The fourth run is the one that stands.** Self-contained: no `RUNPATH` in the
binary at all, `llama-tokenize` runs bare from the host and agrees with the
corpus, the prefix is 95 MB where the shared build was 656, the oracle tier
finds the provisioned prefix on its own and reports 57 of 57 comparisons in
agreement through it, and podman's store is still 195,424 KB. B-367's three
conditions, each exercised rather than asserted: reproducible from its record
(the recipe, the digest, the commit and the package set are all in the
prefix and the journal), removable without residue (done once, through MCF,
with the reason recorded), nothing outside the environment touched.

**Four findings from one command in one evening, none of them about the
component.** A store that follows the wrong variable, a guard that mistrusts a
mount, a path baked in from the wrong side of a boundary, and — implicit in all
three — that a provisioning which *succeeds* can still hand back something that
does not run. D39's four conditions say what a controlled environment must
guarantee; these are what it took to make one do so on one machine with one
component, and each is now either code or a test.

## 32 · F32 — A defect the oracle let through, and the hole it came through (B-364, B-368, F27, A19, A21)

**What prompted it.** B-364 listed quantization schemes MCF decodes that no
acquired file had ever carried. Five small variants of two corpus models were
acquired to change that — 88 to 113 MB each — and their directories read for
what they actually hold:

| file | tensor types carried |
|---|---|
| Llama-160M IQ4_XS | IQ4_XS ×84, Q8_0, F32 |
| SmolLM2 IQ3_XS | IQ3_S ×30, IQ4_NL ×180, Q8_0, F32 |
| SmolLM2 IQ3_M | IQ3_S ×27, IQ4_NL ×120, Q5_0 ×60, Q4_K, Q8_0, F32 |
| SmolLM2 Q2_K | **Q3_K** ×30, IQ4_NL ×180, Q8_0, F32 |
| SmolLM2 Q3_K_S | Q3_K ×30, IQ4_NL ×180, Q8_0, F32 |

Two things a file name does not say: a "Q2_K" of a 135M model carries no Q2_K
tensor at all — the quantizer falls back to Q3_K at this size — and the IQ3
variants are mostly IQ4_NL. The scheme a file exercises is read from its
directory, not its name (§3.18, again).

**Q3_K was wrong, and it read as gibberish.** `asionally himself He
intoosaicunken's.` where the reference says `Paris. Paris is the political,
cultural`. The reference indexes the 32-byte high-bit plane with the value's
position alone, the same bytes serving both halves of the block, the advancing
mask bit telling them apart; MCF indexed it with `half × 32 + position`, ran
off the end for the second half, and `get` handed back zero for every one of
those, which reads as "inverted". F19 had rewritten this decoder against the
reference and given it a test; the test decoded a block and checked the values
were all there, and they were — a plane read off its end is a plane of zeros,
which a test on a zeroed fixture cannot see. One index, and a real file, was
what it took.

**The oracle passed it, through the hole F27 named.** Its rule was: a
divergence is explained if *any* step in the generation had a margin under
0.50. The broken decoder diverged at step 0 with a margin of 0.449 — and was
excused by a 0.021 at step 4, five tokens into text that was already wrong.
Margins across the broken generation: `0.449 1.94 1.33 1.06 0.021 0.226 2.12
0.104`. Against a healthy file's: `0.573 0.248 0.640 3.38 0.105 0.466 1.75
2.93`. No summary of those two rows separates them. What separates them is
*where* they part.

The rule is now: the margin **at the step where MCF's text stops being a
prefix of the reference's**, and nowhere else. `margins --against` finds that
step. Re-measured under it across eleven files, the noise divergences part at
margins from 0.017 to 0.237; the F27 defect parted at 0.775 and this one at
0.449. The threshold moves from 0.50 to 0.30 — above every noise margin
observed, below both defects, nearer the noise — and the remaining hole is
stated in the same words as before: a defect that happens to diverge at a
genuine near-tie still passes. The gap it lives in has narrowed from
0.159–0.775 to 0.237–0.449, which is what more files do to a threshold.

The instrument found one thing about the comparison itself on its first run:
the shell had been folding newlines away where the instrument folded them to
spaces, so a paragraph break read as a divergence one step early. Both now
apply one rule — runs of whitespace are one space — and the rule is written
once in each place with the other named.

**Where this leaves the oracle.** Eleven files, 102 comparisons, all in
agreement or parting under 0.30; with the plane bug reintroduced, the tier
fails on the first prompt of the first Q3_K file — `at step 0, where they
part, the margin was 0.44906, over 0.30 — not a near-tie`. The five witness
files join the corpus tier under the schemes they actually carry.

**The old test could not have found the plane bug**, and the new one fails on
the old code: every plane bit set, every low bit zero, scales that multiply by
one. Correct decoding is 256 zeros; the old decoder produced −4 for the second
half, because a plane read off its end is a plane of zeros. F19's test had
decoded a zeroed fixture, where a plane of real zeros and a plane read off its
end look the same.

**And a scheme nobody had implemented.** IQ3_M refused: `blk.0.attn_v.weight`
in `unknown type 6`, which is Q5_0. Q5_0 and Q5_1 are now decoded from the
reference — the fifth bit of value `j` sits at bit `j` of the block's word for
the first half and bit `j + 16` for the second, which the reference does in one
shift and MCF spells out — with tests that put a bit on values 0 and 16 and
nowhere else.

## 33 · F33 — The last schemes, and the widest noise (B-364, B-368, F32, A21)

**What was acquired.** Four more witnesses, chosen by what their directories
hold rather than what their names say: `SmolLM2-360M Q5_1` (Q5_1 ×224),
`gemma-3-270m Q4_1` (Q4_1 ×126), `gemma-3-270m Q2_K` (no Q2_K at all: Q3_K,
IQ4_NL and Q5_0 — the second "Q2_K" of a small model to fall back), and
`Qwen3-0.6B Q2_K`, the first file with true Q2_K tensors (×112, beside Q3_K
×84). With these, every scheme a corpus file carries — Q2_K, Q3_K, Q4_0,
Q4_1, Q4_K, Q5_0, Q5_1, Q5_K, Q6_K, Q8_0, IQ4_NL, IQ4_XS, IQ3_S — is decoded by
a path a real file exercises and the oracle compares.

**All four cohere, and the oracle flagged one — at 0.320.** Sixteen files, 138
comparisons, one over the threshold: `Qwen3-0.6B Q2_K` on *The opposite of hot
is* parts at step 3 — `The correct statement` against `The problem is` — with
a margin of 0.320, above the 0.30 that F32 set. Both texts begin `cold.` and
the first token's margin is 3.9. Of the file's other two prompts, one agrees
outright and one parts at step 0 — `the city of Paris` against `Paris` — at
0.196, inside the noise range.

Is that a defect? The evidence says no, and it is worth setting out because the
number is close. The Q2_K decoder is structurally the same as the Q3_K decoder
F32 verified, with the one difference that its low plane really is sixty-four
bytes indexed by half. A broken decoder produced gibberish on every prompt
(F32); this file produces coherent, correct text on all three and parts once,
late, on a plausible alternative. And Q2_K is where the arithmetic gap between
the two implementations is *widest*: the reference multiplies two-bit weights
against eight-bit-quantized activations, MCF multiplies dequantized floats, and
a two-bit weight carries the least information to agree about. The largest
noise margin observed until now, 0.237, was on a Q3_K file. That the coarsest
scheme produces the widest noise is what one would predict.

**So the threshold moves to 0.40**, between this noise (0.320) and the smallest
defect (0.449). The gap it lives in has narrowed again — F27 had 0.159 to
0.775, F32 had 0.237 to 0.449, this has 0.320 to 0.449 — and every file added
narrows it further, because noise and defects are measured in the same unit.
That is the limit of a comparison of *texts*: it reads a distribution through
one sample. The next instrument compares logits (B-373), where a defect is a
different vector and noise is the same vector to within arithmetic, and the two
do not share a scale.

A second witness was sought and did not exist: `SmolLM2-360M Q2_K` carries
Q3_K and IQ4_NL and no Q2_K either, and agrees with the reference on one prompt
and parts on two at 0.059 and 0.029 — noise, and a fourth "Q2_K" file that is
not one. `Qwen3-0.6B Q2_K` stands alone as the true witness.

**What was not established.** That the Q2_K decoder is right — only that it is
structurally the verified decoder's twin, coheres on three prompts, and parts
once at a margin the coarsest scheme would be expected to produce. A21 applies:
stated as evidence, not as a verdict, until the logits comparison exists.

## 34 · F34 — Distributions against distributions (B-373, B-368, F27, F32, F33, A19)

**Why.** Three findings in a row moved the text comparison's threshold inside a
gap that closed with every file — 0.159–0.775, then 0.237–0.449, then
0.320–0.449 — because a text samples a distribution once, and the margin that
excuses a divergence is *MCF's own confidence*, which is the thing under test.
The instrument that does not narrow compares the distributions themselves.

**The instrument.** The reference exposes its distribution through one tool:
`llama-server`, whose completion endpoint returns, for every generated token,
the top-N tokens with their log-probabilities (`n_probs`). The recipe gained
that target and the oracle was re-provisioned. For each model and prompt the
tier starts one server on loopback, asks for ten greedy tokens with the top
twenty at each step, and has MCF's `margins --logprobs-of` print its own
log-softmax for exactly those twenty tokens at the same step. At step 0 the
two contexts are the same tokens by construction (the tokenizer section
already holds that); at the step where the texts part, they are compared only
if both engines reached it through identical token ids — text agreement is not
token agreement, and the first version of this compared MCF's step 0 against
the reference's step 9 and reported gaps of twenty.

**Three statistics were measured before one was chosen**, on a clean engine and
on two engines with known defects, over sixteen files and three prompts:

| engine | largest gap, top 20 | largest gap, top 5 | KL over the top 20 |
|---|---|---|---|
| clean (n = 56) | median 0.35, max **1.85** | median 0.21, max **1.12** | median 0.002, max **0.113** |
| rotation swapped (n = 63) | median 2.75, min 1.09 | median 2.03, min 0.34 | median 0.32, p90 2.6, min 0.002 |
| Q3_K plane bug, Q3_K files | ≥ 12.5 | ≥ 12.5 | min 1.59, median 7.53, n=15 |

**What the numbers say, and what they do not.** On every statistic the
rotation defect's *smallest* value sits inside the clean range: at a step where
the model barely attends to position — the second token of a five-token prompt
on a 135M model — a wrong rotation changes the distribution by less than
quantized-against-float arithmetic does. No statistic separates a subtle
defect at every position, because at some positions there is nothing to
separate. What the tier judges is every prompt of every file, and there the
separation is wide: with the floor at a KL of 0.20 — under twice the clean
maximum — the rotation mutant crosses it on the majority of its 63 comparisons
and the clean engine on none of its 56.

KL was chosen over the two gap statistics for the size of that separation
(the clean maximum is one-third of the defect's median; for the top-five gap it
is one-half) and because it is the quantity the register item named: a defect
is a different vector, noise is the same vector to within arithmetic, and a
divergence between distributions is what measures that. The gap statistics are
still printed beside it, so a reader of a failure sees all three.

**Where the noise is.** The clean engine's largest KL, 0.113, is gemma-3-270m
at Q6_K; the next are the mixture of experts at Q5_K_M and gemma at "Q2_K".
The coarse schemes and the small, wide-vocabulary model are where two
implementations' arithmetic differs most, which is what F33 predicted from the
text comparison. The floor is calibrated on this corpus; a file whose noise
sits above 0.113 would be a new measurement, not a defect, and would say so by
which file and which prompt.

## 35 · F35 — What a load costs, apart from running (DEC-018, D41, B-034, §3.13)

**The question.** §7.18 asks whether a served model stays resident, and frames
it as a cold start on every request against memory held. Before deciding, the
two costs were separated on this machine — one reading each, no window, so the
figures are for scale and not for the record's baselines.

**Through the daemon, loading per request, Qwen3-0.6B at Q4_K_M, a one-token
answer:** 6.54, 6.52, 6.49 s. **The same with the model held after the first
request:** 6.66 s (the load), then 12.28 s and 5.63 s resident. Residency did
not make the request faster, and the second resident run was slower than the
load — the machine is shared and nothing here was attributable, which is what
the outlier says and all it says.

**So the load was measured alone.** `load`, a diagnostic that reads, parses and
dequantizes and prints each:

| model | read | parse | dequantize | on disk |
|---|---|---|---|---|
| Qwen3-0.6B Q4_K_M | 0.112 s | 0.041 s | 0.636 s | 397 MB |
| gemma-3-270m Q6_K | 0.080 s | 0.056 s | 0.293 s | 283 MB |
| Llama-160M Q4_K | 0.034 s | 0.007 s | 0.177 s | 121 MB |

Against that, six forward passes through Qwen3-0.6B in this process — a
five-token prompt and one generated token — take 6.8 s. The load is 0.8 s. On
MCF's own engine a request is the forward passes, and the load is the twelfth
part of the shortest one.

**What this decides, and what it does not.** It decides D41: residency is held,
because it costs nothing idle and hides nothing when stated, and because on a
faster engine the proportions invert — but it is not the saving §7.18 imagined
on the engine MCF has today, and pretending otherwise would be a number
without its conditions. It does not decide anything about two models, about
memory pressure, or about an engine whose forward pass is milliseconds; those
are DEC-009's and B-032's, and this finding is the baseline they will be read
against.

**And what residency costs idle: nothing.** The soak tier idles a daemon for
sixty seconds with the fixture resident after one generation and reads zero
context switches and zero clock ticks of processor time — the same figures as
an empty daemon — with the record unchanged. Residency is memory and nothing
else, which is the condition D41 was decided under.

**What was not established.** These are single readings on a shared machine
without the window; the 12.28 s is unexplained and recorded as such. The
budget tier's first-token figure (B-035) is the one taken under conditions,
and it is on the fixture, where the load is microseconds either way.

## 36 · F36 — The reference model answers, through an engine that is a process (B-032, B-033, B-367, D39, §XII)

**What was built.** An engine adapter for the provisioned `llama.cpp` (F31): a
supervised subprocess per generation, its output streamed to the client as it
arrives, its exit classified into the stage it died in — not started
(`engine.spawn.not_found`, `engine.spawn.refused`), dead before a byte
(`engine.exit.immediate`), dead after part of an answer
(`engine.exit.midstream`), killed (`engine.exit.signal`) — and the daemon
standing after each. The laboratory produces every one of those with a shell
that dies the stated way (D26), and the whole-system tier drives a fake engine
through the daemon: chosen without being asked when it is the only one
provisioned, named in the account, its partial answer kept and its own last
words attached when it dies mid-answer.

**The stated rule for which engine serves** (§3.15): what the client asks for;
else the provisioned engine where exactly one is here; else MCF's own. Two
provisioned pins is refused by name rather than chosen between. The account
says which served — `engine: provisioned llama.cpp @925e1179947e from
<prefix>` — and a provisioned answer carries no degraded mark, because it is a
real engine and D39 says a timing taken under stated conditions through it
would be a measurement.

**Two things the first live run found.** The completion tool's `--log-disable`
silences the completion itself in this build: the first provisioned
generation exited cleanly with an empty answer. And `mcf run` was refusing the
reference model *before* asking the daemon, on MCF's own engine's architecture
check — a check that belongs to the engine that will run, and only that one.
Both are fixed and the whole-system tier holds them.

**Then the reference model answered.** `Qwen3.8-27B-UD-Q4_K_M.gguf` — 27.3
billion parameters, 16.5 GB, the artifact §XII named, refused by MCF's own
engine as an architecture it has not been taught and as 109 GB dequantized
against 63 usable (B-372) — through the provisioned engine, on this machine's
processor, twelve tokens:

```
$ mcf run …/Qwen3.8-27B-UD-Q4_K_M.gguf --prompt "The capital of France is" --limit 12 --engine provisioned
 Paris.
The capital of Germany is Berlin.
  produced 12 token(s); stopped: engine_finished
  engine   provisioned llama.cpp @925e1179947e from …/provisioned/llama.cpp@925e1179947e
```

Ten and a half seconds of wall clock, load included, one reading on a shared
machine without the window — for scale, not for the record (B20). The corpus
model through the same engine: `Paris. The capital of France is also the
capital of the`, twelve tokens, 1.7 s.

**What this closes.** M2's first exit criterion in the form §VI meant it: a
model of the size §XII chose for residency and arbitration, on a machine that
had never served it, from one command. B-032 in both halves — the engine is a
recorded condition, and it is a supervised subprocess. B-033 at the stages a
process can die in. And the path D39 opened: what MCF provisions, MCF drives.

**What was not established.** Nothing timed under conditions — the engine may
be timed, and the budget tier does not time it yet (B-035 measures MCF's own
engine on the fixture, on purpose). Residency for a subprocess engine: this
loads per request, and the daemon's resident model is MCF's own engine's; a
long-lived server child is the shape that would hold a 16 GB model between
requests, and that is a further increment of B-032. One engine, one component,
processor only.

## 37 · F37 — The first probe found two defects and then refused to answer (B-051, B-052, D42, §3.18, F25, F26)

**What was built.** `mcf probe`, the framework D42 describes, and the first
configuring probe. Its question is the chat template, and its observation needs
no judgement: a model addressed the way it was trained ends its turn at its own
stop token, and one addressed wrongly runs to the budget. Five trials per
addressing, forty tokens each, through a named engine.

**The first defect: MCF never used the stop token it had been reading.** Every
generation ran to its budget, because `Request.stop` was empty at both call
sites — `mcf run` and the daemon — while the vocabulary had been carrying
`ending` since the tokenizer was written. A model saying *I am done* was
generated as an ordinary token and passed over. Nothing had noticed, because
nothing had asked *why* a generation ended; the probe asks exactly that, and
found it in its first run. Both sites now pass the model's own end of text.

**The second defect: the answer was an artifact of the instrument.** With the
stop token honoured the probe decided, and said `raw` was best for both
SmolLM2 and gemma3 — with `turns 0 of 5` against `raw 5 of 5`. That is a clean
result and it was worthless. MCF refuses to parse control tokens out of prompt
text — a prompt must not be able to produce a chat marker by spelling one
(F26) — so `<|im_start|>` written into a prompt reaches the model as **eight
ordinary tokens**, and `<start_of_turn>` likewise. The chat addressings were
never applied. The probe had compared raw against a garbled prompt and reported
the garbling as the model's behaviour.

This is F25's lesson arriving from the other side. There, a broken engine read
*better* than a correct one; here, a broken experiment produced a *cleaner*
result than an honest one — `5 of 5` against `0 of 5` is the most decisive
table in this document and it means nothing.

**So the probe now checks that what it sends is what it means.** Each marker is
tokenized and must come back as one token spelling itself. Where it does not,
the addressing is untestable *as text*, and the probe reports inconclusive
naming exactly why. On the corpus today that is every instruct model:

```
SmolLM2-135M   INCONCLUSIVE — chatml cannot be applied: its markers are control
               tokens, and MCF does not parse control tokens out of prompt text
               (F26) … addressing has to be built from token identifiers (B-374)
gemma-3-270m   INCONCLUSIVE — turns cannot be applied: …
Llama-160M     INCONCLUSIVE — no addressing ended at the model's own stop token
               within 40 tokens; a larger budget may decide it
```

**A probe that will not answer is the probe working.** D42 made *inconclusive*
a first-class outcome for exactly this: *the model did not do the thing* and
*MCF could not tell* are different facts, and only the first would license a
configuration. What MCF has learned here is about MCF — its prompt surface
cannot express a chat turn — and B-374 is the item that fixes it: addressing
built from token identifiers rather than from text, which is what a chat
template actually is.

**What was not established.** Nothing at all about how these models prefer to
be addressed; that is what B-374 unblocks. The stop-token fix is verified only
in that generations now end at a model's own end of text where they did not
before — how often, on which models, under which addressing, is the same
unasked question until the probe can ask it.

## 38 · F38 — The probe's answer was upside down, and the decisive column was the broken one (B-052, B-374, D42, §3.18, F25, F37)

**What was built.** B-374: a chat turn assembled from token identifiers rather
than from text. A request now carries either a prompt or a list of identifiers;
MCF builds the identifiers from the model's own markers, taken from the
template in its own file; and what a *user* typed is never among them. This is
what F26 left unbuildable and F37 stopped on — with it, the chat-template probe
could compare addressings for the first time.

It compared them, decided, and was wrong in the exact opposite direction.

**The observation counted silence as success.** SmolLM2-135M, every one of its
five quantizations, reported the same table:

```
im_start…im_end as assistant 0 of 5
raw                          5 of 5 ←
```

The addressing was verified correct before this was believed — the identifiers
were dumped and read back:

```
["<|im_start|>", "user", "Ċ", "What", "Ġis", "Ġthe", "Ġcapital",
 "Ġof", "ĠFrance", "?", "<|im_end|>", "Ċ", "<|im_start|>", "ass", "istant", "Ċ"]
```

That is ChatML, exactly. So the two addressings were sent by hand and what came
back was read rather than counted:

| addressing | tokens produced | ended at | what it said |
|---|---|---|---|
| ChatML | 40 | the budget | "The capital of France is Paris. Paris is a city located in the northern part of the country…" |
| raw | **0** | its own stop token | *nothing* |

The addressing the model answers under scored **zero**. The addressing it
refuses to speak under scored **five of five**. A model addressed in a way it
does not recognise emits its end-of-turn token *first thing* — and the probe's
question, *did it end at its own stop token*, cannot tell that apart from a
finished answer. It is the same token. The whole result was inverted.

**Why it was believed for as long as it was.** Because it was consistent. Five
quantizations of the same model, `5 of 5` against `0 of 5` every time — the
kind of stability that reads as signal. F25 is the same lesson from the other
side (a deleted MoE router read *better* than the correct one) and F37 the same
again (a garbled prompt gave a cleaner table than an honest one). Three times
now the decisive-looking result has been the broken one. **Consistency is not
correctness, and a clean margin is not evidence about anything but itself.**

**The fix is not a threshold.** A trial counts only if the model *said
something and then stopped*: `Trial::Stopped` carries how many tokens preceded
the stop, and zero is a refusal to speak, not a completed turn. The boundary is
not arbitrary — nothing said is nothing said. The silences are kept and printed
rather than folded into the failures (A1), because *the model recognised its
stop token and declined the turn* is a fact worth having:

```
im_start…im_end as assistant 5 of 5 ←   turn ran 10-292 token(s), middle 96
raw                          1 of 5     (ended without saying anything 4 of 5)   turn ran 120
```

That is SmolLM2 after the fix, and it agrees with the file's own declaration.

**The turn lengths are kept, because they were already paid for.** A trial
cannot tell a finished turn from a refusal without counting what preceded the
stop (that *is* the fix), so the count exists whether or not it is recorded —
and throwing it away would be discarding a measurement MCF already made (A1).
It is the observation a stop-condition question is asked of (B-056), and the
first candidate for separating addressings that tie on *did the turn end*
(B-375). The spread above is worth noticing on its own: the same model, the
same addressing, five short factual questions, and turns from 10 tokens to
292. Any budget chosen for *this* probe by looking at one question would have
been wrong for the others — which is how the budget came to be measuring
verbosity in the first place.

**A third defect fell out of the first two: five trials were one trial.** MCF
samples greedily from a fixed seed. Five trials of one question are one
observation written down five times, and `5 of 5` was arithmetic wearing the
costume of evidence (A19). The probe now asks a different short question each
trial, so five trials are five observations of the thing actually being
asked — *does this addressing get an answer out of this model*.

**And the budget was measuring the model's verbosity.** At forty tokens the
correct addressing had not finished; at 160 it still had not; it ends its turn
cleanly at **250**. A budget too small to reach the end of a turn makes every
addressing look identical, so *ran out of budget* was doing the work that *did
not stop* was being credited for. It is 320 now, and B49 holds — the budget is
in tokens, not seconds.

**gemma-3-270m stays inconclusive, and correctly.** Under the broken
observation three addressings tied at `5 of 5`; once silence stopped counting
and the budget was large enough to reach the end of a turn, the tie narrowed to
**two** — so one of the three had been tied on refusals. Two addressings answer
and end the turn equally often, and this observation cannot tell those two
apart. Their
turn lengths appeared to separate them, and **that separation did not survive a
change of engine** — see F39, which measured the same thing through the
provisioned server and found the two addressings running 7-11 and 8-11 tokens
where MCF's own engine had them at 6-91 and 7-10. B-375's first candidate is
therefore not yet a candidate: what it separated on one engine it does not
separate on another, which is the definition of a reading that characterizes
the instrument (B29). The
tie-break that preferred `raw` was itself a defect — `max_by_key` returns the
*last* maximum, which handed every tie to whichever candidate was listed last,
and `raw` always is. A tie is now `inconclusive` naming the tie, because
choosing on a tie would be MCF reading its own default back as a finding
(§3.15, D42).

**What this puts within reach.** M3's first exit criterion is *a model that the
defaults configure wrongly measurably improves, and the improvement is
attributable to a named probe*. SmolLM2 is that model, and the margin is not
subtle: addressed the way MCF addresses models today it produces **nothing at
all**, and addressed the way the probe found it produces a fluent answer. What
is missing is not the evidence but the act — nothing here configures anything
(D42), and the parameter that would carry *which probe, when, under what
conditions* is B-059.

**What was not established.** Whether the addressings gemma3 ties under are
genuinely equivalent for it, or whether a sharper question separates them —
that is the next increment, and the probe says so rather than guessing. Nothing
about any model larger than 270M: both models here are small on purpose, and
what a probe learns on a 135M model is about that model. And nothing is
configured by any of this — `mcf run` still sends raw text (§3.8); D42 holds
that a probe writes the verified half of a capability and never a default.

## 39 · F39 — The engine that can be probed, and three ways the instrument stood in for the model (B-032, B-055, B-376, D39, D42, F36, F38)

**What was built.** The provisioned engine driven as a **server** rather than
as a completion tool. F36's engine is a subprocess per generation — a command
line in, text out — and it runs the reference model honestly and cannot be
probed at all, for two reasons that are properties of the interface rather than
of llama.cpp: a turn of token identifiers has nowhere to go on a command line
(B-374, F26), and the tool does not say why it stopped. Those are exactly the
two things F38's observation is made of, so every probe was confined to MCF's
own engine.

**The contract was confirmed before anything was written.** The sixteen
identifiers MCF assembles came back as `tokens_evaluated: 16`, and the
generation ended `stop_type: "eos"`. Both halves, in one request.

**It listens on a Unix socket, not a port.** `--host` binds one when the
address ends in `.sock`. Four projects share this machine (§XVII); a port is a
machine-wide resource two of them can collide over, and nothing here listens on
the network. The model stays loaded between requests, which is the residency
F36 left open.

**The instrument stood in for the model three times, in one afternoon.**

*First: ready is three conditions deep.* The socket file appears before
anything listens; the listener accepts before the model is loaded; and a
request in between is answered — with an error, in a shape close enough to a
completion to be mistaken for one. The first readiness check waited for a
connection to be accepted, which is the second condition and not the third, and
the probe's opening trial read a still-loading server as *an engine that does
not say why generation ended*. The wait is now on the server's own `/health`
saying `ok`.

*Second: the fix for F38 was itself engine-dependent.* F38 made a trial count
only if the model spoke first, and *spoke* was measured by counting the token
lines in the stream. MCF's own engine streams one line per token; the server
streams the whole answer as one. So through the server **every** addressing
looked like a one-token turn, and the probe reported a tie where there was
none. The count now comes from the account, which both engines fill in the
same units. **The defect F38 is about, inside the fix for F38.**

*Third: the two engines disagree by exactly one, at the boundary the probe
turns on.* Asked something it does not recognise, a model emits its end-of-turn
token and nothing else. MCF's engine calls that **0** tokens; the server calls
it **1** — it counts the end-of-turn token itself. Neither is wrong. But *said
nothing* is the whole of F38's fix, and a probe taking either literally reports
a refusal on one engine and an answer on the other for one behaviour. The form
both agree on is the **text**, which is empty either way, so a turn with no text
counts as no tokens whatever the engine calls it.

**Then the two engines were made to answer the same question.** This is the
test of whether a probe result is a property of the model or of the instrument
(B29), and it is why the default was not changed on the speed alone:

```
                                       provisioned server        MCF's own engine
im_start…im_end as assistant           5 of 5  ←                 5 of 5  ←
raw                                    1 of 5, silent 4          1 of 5, silent 4
best                                   im_start…im_end           im_start…im_end
wall                                   13 s                      267 s
```

The verdict agrees — the same best addressing, the same counts, the same
silences — and the server is **twenty times faster**. The agreement is what
makes changing the default honest; the speed is what makes probing usable at
all. A 270M model that took tens of minutes takes eight seconds.

**And where they disagree, the disagreement is the finding.** gemma-3-270m's
tie stands on both engines, but the turn lengths F38 recorded as B-375's first
candidate do not:

```
                                         MCF's own engine      provisioned server
start_of_turn…end_of_turn as assistant   ran 6-91              ran 7-11
start_of_turn…end_of_turn as model       ran 7-10              ran 8-11
```

On one engine the two look separated; on the other they look identical. **A
reading that changes with the instrument is a reading about the instrument.**
B-375's candidate is withdrawn rather than kept with a caveat, and F38 is
corrected where it recorded the separation as though it were about the model.

**What this cost, and what caught it.** The repository's own gates caught four
things this change would otherwise have shipped without: two failure categories
claimed with no laboratory scenario able to produce them (A13, B-010), a place
MCF starts a process that was not declared (§6.4), and two deletions that were
not declared (§3.11). None of them were found by the author. That is what those
checks are for, and it is the second time in this document that the gate has
been the thing that noticed.

**What was not established.** Nothing about a large model through this path:
every figure here is a 135M and a 270M model, chosen because a probe that takes
four minutes does not get run. Whether the server holds a 16 GB model usefully
between requests is the thing residency was built for and is not measured. The
turn lengths are now known to differ between engines and it is not known
*why* — whether the sampling diverges, the tokenizers differ at some position,
or something else — and B-373's oracle is the instrument for asking. No timing
here is a measurement: none of it was taken in the exclusive window (B35), and
the twentyfold figure is an order of magnitude rather than a number.

## 40 · F40 — MCF's engine agrees with the reference for seven hundred positions, and the rule that would have called it broken was the wrong rule (B-368, B-373, B-377, D39, A19, F27, F32, F39)

**The question.** F39 found that MCF's engine and the provisioned one produce
the same verdict and different text: the same model, the same identifiers,
greedy from the same seed, generations that agree for a few tokens and then
part. Two explanations, opposite consequences. Either two near-tied tokens are
picked differently and the trajectories diverge from there — which is nobody's
defect — or the distributions genuinely differ, and every result MCF has ever
measured through its own engine is suspect.

**What the oracle could already say, and where it stopped.** `margins
--against` reports the margin at the step where MCF's text stops being a prefix
of the reference's, which is F32's fix for F27's rule. On SmolLM2 the two part
at **step 4 with a margin of 0.105** — a near-tie, well under the 0.40
threshold. That is the benign answer, and it is only an answer about step 4.

**The oracle has only ever compared ten tokens.** `GENERATE_TOKENS=10`. Past
the parting step nothing is comparable, because the two engines are writing
different sentences — so *whether agreement decays with position* had never
been asked, and position is exactly where a rotary encoding or a cache would go
wrong.

**So MCF was made to read the reference's tokens instead of its own.** Teacher
forcing: at every position MCF sees exactly the reference's prefix and is asked
what comes next. That is comparable all the way down, however far the free
generations have drifted apart.

| | SmolLM2-135M | gemma-3-270m |
|---|---|---|
| positions compared | 250 | 700 |
| disagreements | 3 (1.2%) | 25 (3.6%) |
| worst rank MCF gave the reference's token | 1 | 3 |
| largest margin at a disagreement | 0.113 | 6.590 *(see below)* |

**The sliding window holds when the cache is grown a token at a time.** F28
already crossed this boundary and established the mask against the reference —
with a **682-token prompt**, including one built so that only an engine masking
the sliding blocks and not the global ones recalls a fact placed outside the
window. What it did not exercise is the other path to the same state: a cache
filled *one position at a time by generation* rather than in a single pass over
a prompt. This does that, and compares 700 positions rather than the eight
generated tokens F28 compared. Across the boundary:

```
positions   0-255   disagreed 17 of 247   (6.9%)   worst rank 3
positions 256-511   disagreed  8 of 256   (3.1%)   worst rank 1
positions 512-700   disagreed  0 of 189   (0.0%)   worst rank 0
```

**One hundred and eighty-nine consecutive agreements past the window.**
Agreement does not decay with position; on this model it improves.

**The one alarming number was the instrument again.** Position 385 showed a
disagreement at a margin of **6.59** — an order of magnitude past the largest
real defect this project has recorded (0.775, F27). MCF's chosen token there
was **106**, which is gemma's `<end_of_turn>`. The reference had been run with
`ignore_eos` so that it would produce seven hundred tokens, so it was forbidden
to stop and took its best remaining token while MCF took the stop. The margin
was measuring a flag in the experiment. **That is the third time in three
findings that the most decisive number was the one the instrument made** (F37,
F38, F39), and the only reason it did not become a defect report is that the
token was looked up rather than the number believed.

**Two disagreements were left, and they split the two rules apart.** At
positions 45 and 160 MCF preferred a different token with margins of 0.490 and
0.704 — both over the oracle's 0.40 threshold, both in the range where F27 and
F32 found real defects. The distribution comparison, which is the oracle's
other and better instrument (B-373), says otherwise:

| | position 45 | position 160 |
|---|---|---|
| top-20 sets in common | 19 of 20 | 19 of 20 |
| KL(reference ‖ MCF) | **0.028** | **0.068** |
| threshold for *distributions differ* | 0.20 | 0.20 |
| largest single log-probability gap | 0.362 | 0.565 |

The two engines hold the same twenty tokens with nearly the same probabilities
and order the top two differently. At position 160 the reference itself has its
top two **0.06 apart** — its own near-tie — where MCF has them 0.70 apart the
other way.

**The finding about method: the margin rule does not transfer, and the KL rule
does.** The 0.40 threshold was measured on *one parting step per file*, across
sixteen files (F27, F33). Applied at every position of a seven-hundred-step
comparison it is a different test with a different rate of false alarm, and it
raised two. The distribution rule, on the same two positions, says agreement
with room to spare. Nothing about the gating threshold changes — it is still
right for the quantity it was calibrated on — but **the per-position test must
use the distribution rule, and B-373's case for preferring logits over texts
is stronger than when it was written.**

**What was established.** MCF's engine agrees with the reference across 250 and
700 positions of two different architectures, one of them with sliding-window
attention exercised past its own window for the first time. Every disagreement
is an order-swap between tokens both engines hold at nearly the same
probability. The divergence F39 asked about is near-tie amplification, and
B-377 closes on that.

**What was not established.** Whether a long *prompt* and a long *generation*
could ever disagree here — F28 covered the first path and this the second, and
neither has been shown to be the harder one. Nothing about a large model: both are under 300M,
and a defect that only appears at 27B would not show here. Nothing about
quantizations other than Q8_0 and Q6_K — F33 measured the arithmetic gap
widening at Q2_K, and this test has not been run there, where a 0.56
log-probability gap might be ordinary or might not. Nothing about a *prompt*
longer than the window: this exercises long generation, and a long prompt fills
the same cache by a different path. And no timing: the seven-hundred-position
comparison took five minutes and thirty-seven seconds of a shared machine
outside the exclusive window, which makes it a duration and not a measurement
(B35).

## 41 · F41 — The check could not fail, and the mutation that showed it was not the first one tried (B-003, B-368, B-377, A13, F40)

**What was added.** F40's teacher-forced comparison, made durable as a section
of the oracle tier: MCF is made to read the reference's own tokens, and what is
asserted is the *rank* MCF gave the reference's token at each position. A
top-two order swap is arithmetic; a reference token MCF ranks tenth is not.
Off by default at `MCF_ORACLE_FORCED=1`, because it costs minutes per model.

It passed on the first run — 354 of 377 positions on gemma-3-270m, the
reference's token never worse than MCF's rank 3 — and the number meant nothing.

**The negative control returned identical figures with the mechanism broken.**
B-003's rule is that a check has to be shown to fail. The sliding window was
deliberately given an off-by-one and the section reported *the same numbers to
the digit*: 354 of 377, worst rank 3. Not a near miss — no difference at all.

**The reason is that the check never reached the mechanism.** gemma-3-270m
attends over a window of 512 tokens and stops at its own end of turn after
**377**. A window of 512 is never reached in 377 tokens, so nothing about the
sliding window was being exercised, and the check was asserting a property of
the first 377 positions while its comment claimed the window. (F28's long-prompt
comparison does cross the boundary and always did; what was empty here was this
section's own claim to cross it by generating.)

This is why F40 used `ignore_eos`, and F40's own artifact is why the tier had
not: a reference forbidden to stop takes its best remaining token where MCF
takes the model's end of turn, and that produced the largest apparent defect in
this document. The two are not in tension once the artifact is *named* rather
than avoided — the flag is set, and the positions where MCF chose the stop
token are set aside by that name and counted in the output (A1). On this model
that is exactly one position in seven hundred.

**Then the mutation was still too weak.** With the run reaching 700 positions,
the off-by-one *still* changed nothing. That is not a failure of the check but
a fact about the model: the extra key is at the far edge of the window and
carries a negligible attention weight. A boundary error of one token in this
mechanism is, on this model, unobservable.

**The structural break is caught, enormously.**

```
                              agreed        worst rank of the reference's token   verdict
clean                         675 of 699    3                                     passes
sliding window off by one     675 of 699    3                                     passes  ← not caught
sliding window disabled       462 of 700    618                                   fails, exit 1
```

Rank **3** against rank **618**. The threshold sits at 8, in a gap of two
orders of magnitude — which is the widest separation between noise and defect
anything in this document has measured, and a good deal wider than the margin
rule's 0.320-to-0.449 (F32, F33).

**What this check does and does not cover, stated rather than implied.** It
catches a sliding-window mechanism that is absent, wrong in shape, or applied
to the wrong blocks. It does **not** catch a boundary off by one, and that is
now known by measurement rather than assumed either way — which sharpens F28,
where an off-by-one was reasoned about as *a defect with room to hide* and a
history was built to deny it that room. On this model it hides regardless: the
key at the window's edge carries too little weight to change the answer. A21 applies to MCF's
own instruments as much as to a model's metadata: what a check has been shown
to catch is what it verifies, and the rest is declared.

**What was not established.** Whether the off-by-one is unobservable on other
models, or only on this one — a longer window, or a model that leans harder on
its oldest visible token, might make it plain. Nothing about models without
sliding-window attention, where this section still compares 700 positions but
tests no windowing at all. And the run costs five to six minutes per model on a
shared machine, which is why it is off by default and why nothing here is a
timing (B35).

## 42 · F42 — The context both models declare is the context they have, and the probe that asked broke the protocol asking (B-055, B-058, B-059, B-376, D42, §3.7, §3.8, A2, A21)

**What was built.** The second probe: the context length a file declares
against the longest prompt the engine will actually take. It is reachable only
because of B-376 — MCF's own engine pays a forward pass per prompt token, so a
32768-token question through it is hours, and through the server it is thirty
seconds.

**What can be observed here without judgement.** Not *does the model still
understand the context*, which is a judgement and is not built. The exact
observation is an integer comparison: MCF sends N identifiers and the engine
reports how many it read. Equal is agreement; fewer is a prompt silently
shortened, which is a measurement of a different prompt (§3.8, D46).

**The claim holds on both models, and the shape of the claim is worth stating.**

| | declared | accepted as prompt |
|---|---|---|
| SmolLM2-135M | 8192 | 8191 |
| gemma-3-270m | 32768 | 32767 |

The off-by-one is not a divergence and is not reported as one: **a declared
context is the whole budget, not the prompt's share of it.** A prompt of
exactly 8192 is refused because nothing is left to answer with. Reporting that
as a defect would be reporting arithmetic, and the probe leaves one token for
the reply and says so.

**The claim is asked directly, so agreement costs one trial.** A halving search
runs only when the claim fails. On the corpus that is one question and no
search — 8191 tokens spent rather than fifteen trials' worth.

**The divergence branch was exercised deliberately, because a check that has
only ever agreed has shown nothing** (B-003). The engine was given a 2048-token
context against a file declaring 8192:

```
declared 8192 token(s)
accepted 2047 token(s) of prompt, with one left to generate
DIVERGENCE the file declares 8192 tokens and this engine on this machine takes 2047
the engine's own words: request (2048 tokens) exceeds the available context size (2048 tokens)
```

Found exactly, in fourteen trials.

**Asking the question broke the control protocol, which is the finding.**
`REQUEST_CEILING` was sixty-four kibibytes, on the stated reasoning that *a
control request is a verb and a name*. That was true when it was written and
stopped being true at B-374, which made a request able to carry a turn of token
identifiers. Thirty-two thousand of them written as decimal numbers is a
quarter of a megabyte, so a **legitimate request was cut off at about four
thousand tokens**.

Worse than the bound was what happened at it. The daemon read its sixty-four
kibibytes, failed to parse the fragment, and closed while the client was still
writing — so the client saw a connection reset and **no reason at all**. The
probe reported *a line of the stream was unreadable*, which was true and
useless. That is the silent failure A2 forbids, in the one place §3.7 says to
be careful.

Both halves are fixed. The ceiling is four mebibytes, derived from the largest
thing a request can honestly be — a turn of identifiers for a very long
context — and is still a stated number rather than *whatever arrives*, which is
what §3.7 actually asks for. And a request that fills the ceiling without
ending is now **refused with the ceiling named**, before the connection closes.

**A taxonomy error the divergence made visible.** MCF classified the server's
`400 exceeds the available context size` as `engine.protocol.malformed`,
attributed to the machine. The server's answer was perfectly well formed; it
refused a request that was out of bounds and said exactly why. Calling that
answer unparseable blames the engine for the request. It is `config.invalid`,
refused, attributed to the request — and `malformed` is kept for an answer that
genuinely cannot be read. The wrong category was written by the same hand that
wrote the scenario for it (F39) and survived until a real refusal arrived.

**A condition that named the wrong experiment.** The probe framework built its
conditions with `CHAT_TEMPLATE.name` hard-coded, so the context probe's result
carried `probe: chat-template` — a provenance field, printed beside the true
ones, stating the wrong experiment. It is the exact failure B-059 exists to
prevent, and it existed for as long as there was only one probe to be wrong
about. The method is a parameter now.

**What was not established.** Whether a context that is *accepted* is a context
that is *usable* — a model may take 32767 tokens and attend to none of them,
and that question needs an observation this probe does not make. Nothing about
memory: both models are small enough that their declared context fits, and a
27B model at 32768 may not, which is the case where this probe earns its keep
and has not been run. Nothing about MCF's own engine, which cannot answer here
at all: it does not report how many identifiers it read, so the probe returns
*could not tell* rather than assuming it read them all (A7). And no timing —
thirty seconds on a shared machine is a duration, not a measurement (B35).

## 43 · F43 — A model that produced nothing now answers, and the change can be accounted for (B-059, B-062, D42, D43, §3.8, §3.15, A21, F38)

**M3's first exit criterion, met.** *A model that the defaults configure
wrongly measurably improves, and the improvement is attributable to a named
probe.* The same model, the same prompt, the same engine, either side of one
act:

```
BEFORE  mcf run … --prompt "What is the capital of France?"
        [end of text]

THE ACT mcf probe … --apply
        APPLIED  im_start…im_end as assistant — set by the chat-template probe
                 at 2026-08-27T21:32:11, through provisioned (MCF 0.1.0-m0)

AFTER   mcf run … --prompt "What is the capital of France?"
        The capital of France is Paris. Paris is a city located in the northern
        part of the country, known for its historical landmarks, cultural
        institutions, and cultural attractions…
```

The margin is not subtle and it is not a matter of quality: addressed the way
MCF addresses every model today, SmolLM2 emits its end-of-turn token and
**says nothing at all**. F38 measured that as a refusal to speak and it is what
`mcf run` had been doing since there was an `mcf run`.

**What the act is, and what it is not.** D42 is that a probe reports a
measurement and configures nothing; D43 is that MCF never reconfigures under a
user. Between them there has to be a person, and `--apply` is where the person
is. It is a flag rather than a default because that *is* the decision: MCF may
learn better, and what it does with that is say so until somebody asks.

Three answers, and only one writes anything. Observed and not raw: written down
with the probe, the moment, the build and the conditions, and the act goes on
the record as a `model_configured` entry naming what MCF did **before**, so the
line says what changed rather than only what is now true. Observed and raw:
nothing to apply, because writing a configuration that changes nothing would
put a probe's provenance on a default and make it look derived (A21).
**Inconclusive: refused** — and gemma-3-270m is the case, refused in a real run.
D42 made *could not tell* first-class exactly so that it could not become a
configuration, and this is the place that rule has to hold or it holds nowhere.

**Every value answers *why this value*, and answers it where it is used.** The
provenance is not filed away; it is on the account, in the record, and on every
`mcf run`:

```
  addressed im_start…im_end as assistant — set by the chat-template probe
            at 2026-08-27T21:32:11, through provisioned (MCF 0.1.0-m0)
```

§3.15 is why it is printed rather than merely stored: MCF doing something other
than the plain thing must never be something a reader has to go looking for.
The full build identity and the conditions in full are in the file and the
journal entry, where somebody chasing a difference between two machines will
look — legibility decides where they are shown, A1 decides that they are kept.

**The test that makes the attribution real.** *Attributable to a named probe*
is only true if the thing applied is the thing measured. A configuration that
rebuilt the turn slightly differently — another marker, a lost newline — would
be a different addressing wearing the probe's provenance, which is worse than
no provenance. So the winning addressing is carried out of the probe rather
than looked up again by name, and a test asserts that the turn a stored
configuration builds is the turn the probe sent, **identifier for identifier**.

**What the gate caught, again.** `forget` destroys a file and §3.11 requires
every deletion in MCF to be declared with what it destroys and why that is not
an artifact. It was not, and the gate said so. What goes is MCF's note about
how to address a model — never the model, and never the record: the
`model_configured` entry outlives the file the way `ArtifactRemoved` outlives an
artifact.

**What was not established.** The other half of D43: MCF does not yet notice
when its answer *would now differ* from what was applied — a better probe,
another engine, a changed default — which is the divergence half of B-058 and
is not built. A configuration is therefore as good as the day it was taken, and
nothing warns when it stops being. Nothing about a second parameter: the usable
context is probed (F42) and is not applied to anything, because nothing in MCF
yet reads a context bound from configuration. And the improvement here is one
model on one machine through one engine — §3.8's point is that the *wrong*
configuration corrupts a measurement, and this shows the mechanism, not a
general result about small models.

## 44 · F44 — A configuration says whether it still holds, and the probe that asked spent eight thousand passes learning it could not (B-058, B-059, D42, D43, A21, F39, F42)

**What was built.** D43's other half: MCF noticing when its answer *would now
differ* from what somebody applied. Two things are checkable and they are not
the same kind of knowledge, so they are reported separately and never mixed.

**The conditions can be checked with no trials at all.** A configuration
records the engine and the build it was taken through, and MCF knows which are
in force. When they have moved it says so, naming both sides:

```
moved    the engine it was taken through is not the one in force:
         was provisioned, now stand-in
         which does not mean the answer changed — two engines agreed on this
         question when it was measured (F39) — only that the evidence was
         gathered elsewhere (A21)
```

**That caveat is the finding, not decoration.** F39 measured two engines
returning the same verdict on this exact question, so *the conditions moved*
and *the answer changed* are different claims and only the first is known. A
tool that reported a changed engine as a disagreement would be manufacturing
divergences, which is the same error as suppressing them and easier to make.

**The answer is compared only where there is evidence.** `mcf probe` has just
measured the thing that set the configuration, so the comparison is free and it
is the one that can say *wrong*:

```
applied  im_start…im_end as assistant — set by the chat-template probe …
agrees   this run measured the same addressing that is applied, so the
         configuration is not merely old — it is confirmed (A21)
```

and, with the stored answer made to differ:

```
DIVERGENCE what is applied is some other addressing, and this run measured
           im_start…im_end as assistant as best. MCF's answer would now differ,
           which is the case D43 is about — applying it is an act
```

**An inconclusive re-probe is not a disagreement.** It leaves the capability
where it was and does not license undoing anything — the same rule as D42's,
applied to the second occasion it matters.

**The probe spent eight thousand forward passes to learn it could not answer.**
Running the whole of `mcf probe` through MCF's own engine took the
usable-context probe (F42) down a path where the answer was never available:
MCF's engine does not report how many identifiers it read, so the probe could
only ever return *could not tell* — after sending it 8191 identifiers and
paying a forward pass for each. The instrument is now asked the cheapest
question there is, **one token**, before the model is asked anything:

```
INCONCLUSIVE — this engine does not say how many identifiers it read, so a
prompt taken whole cannot be told from one quietly shortened (B-376)
```

Same answer, 8190 forward passes cheaper. The general shape is worth keeping:
**ask the instrument whether it can answer before asking the model** — §3.18
makes a probe an experiment, and an experiment whose instrument cannot read the
result is one that should not be run.

**A comparison that invented a parse.** The first version of the build check
took the first two words of the build identity as its version. It worked on the
real string and turned a compiler version into a version number on anything
else. The whole identity is compared now and only the display is shortened: a
different compiler or target *is* a different build, and choosing which
differences count would be MCF deciding where it has no evidence (§3.15).

**What was not established.** Whether a moved condition ever *does* change the
answer here — F39 says not for these two engines on this question, and that is
two engines and one question. Whether the model file itself changed: the
configuration is keyed by path, and a file swapped underneath would be found by
a re-probe and by nothing else. And the divergence is reported where somebody
runs `mcf probe`; a configuration nobody re-probes is still as good as the day
it was taken, which is now *visible* rather than fixed — which is what D43
argues is the honest state and §7.25 feared was a rotting branch.

## 45 · F45 — The page whose job is to have no hidden choices had one, and the check for a moved condition invented one (B-058, B-059, B-062, D43, §3.15, A21, F44)

**What was built.** `mcf explain` carrying the derived configuration beside the
declared defaults. It is the command whose entire purpose is §3.15 — *no hidden
choices* — and since F43 it had been omitting the one choice somebody made
deliberately. A model configured yesterday was explained as though it would be
addressed raw.

```
addressed as   im_start…im_end as     applied by somebody, on a probe's
               assistant — set by the evidence, under the conditions in
               chat-template probe at force here (D43, B-059)
               2026-08-27T22:08:33,
               through provisioned
               llama.cpp @925e1179947e
```

It also said *Nothing here has been probed* under **What is it good at?**, to a
reader who had probed it. That sentence was true when it was written and is the
kind of stale line that makes a reader stop believing the rest of the page.

**The comparison invented a moved condition.** `explain` reported that the
configuration had been taken under conditions that no longer held, on a machine
where nothing had changed. The two sides were not the same kind of name: the
probe recorded the engine as **the name a caller asked for** — `provisioned` —
and `explain` held **the engine that resolved** — `provisioned llama.cpp
@925e1179947e, from /path`. Comparing those compares two spellings of one
engine and finds them different every time.

This is exactly the failure F44 argued the design must avoid, arriving one
commit later by a different route. There, the danger was reporting a moved
condition as a disagreement; here it was reporting a condition as moved when it
had not. **Manufacturing a divergence is the same error as suppressing one, and
it is the easier one to make** — a false divergence looks like diligence.

**The fix is to record the engine as the thing it is.** `provisioned` is not an
engine; a build at a commit is. Both sides now write
`provisioned llama.cpp @925e1179947e`, which also makes a *different build of
the same engine* visible as the moved condition it genuinely is — a distinction
the old name could not express at all. Two tests pin the spellings together, so
that changing one has to change the other.

**A table that assumed its values were short.** The value column was never
wrapped, on the reasoning that a value is a word or a number. True of every row
for as long as there were only defaults; a derived configuration's value is a
*sentence*, because it carries its own provenance. An overrunning value pushed
its source onto the same line and the table stopped being a table. Both columns
wrap now.

**What was not established.** Whether the resolved engine name is stable across
provisioning — a rebuild at the same commit produces the same name and a
genuinely different environment, and MCF would call that unmoved. The
provenance is compared, not the environment it names, and closing that gap
means comparing the provisioned component's own record rather than its name.
Nothing about a second derived parameter: the usable context is measured (F42)
and nothing consumes it, so `explain` has one derived row and the machinery for
n. And the divergence is still only reported where somebody looks — a
configuration nobody explains or re-probes is unexamined, which D43 argues is
the honest state rather than a defect.

## 46 · F46 — MCF's own default was cutting every answer off, and a gate test that depends on whether a daemon is running (B-056, B-059, D42, D43, §3.8, §3.12, B49, F38)

**What was built.** The stop-condition probe, and the second *applied*
parameter — the thing M3 was measured to lack (F45).

**The question it answers, and why it is a configuring probe.** MCF allows
**32 tokens** unless told otherwise. SmolLM2's turns, addressed the way it asks
to be, run to **313**. Every answer past the thirty-second token was being cut
off by MCF rather than finished by the model, which is a measurement of the
budget and not of the model — §3.8's exact complaint, sitting in MCF's own
default the whole time.

```
ended its own turn in 5 of 5 trials, the longest running 313 token(s)

DIVERGENCE MCF allows 32 tokens unless told otherwise, and this model's turns
           run to 313. Every answer past that is cut off by MCF rather than
           finished by the model (§3.8)

APPLIED    313 tokens — set by the stop-conditions probe at 2026-08-27T22:15:34,
           through provisioned llama.cpp @925e1179947e
```

Afterwards, the same question ends at **65 tokens with the model's own stop
token** where MCF's default would have truncated it.

**Telling *does not stop* from *the budget was too small*.** They look
identical from outside, and B-056 is the item that says so. The probe doubles:
32, 64, 128… until the turn ends or a ceiling is reached. Doubling rather than
one large budget because a budget sized for the worst case is spent on every
trial including the ones that finish in ten tokens, and tokens are what a probe
costs (B49). Reaching the ceiling is reported as **not within this many
tokens** — never as *never* — with the number, and naming the likelier cause: a
model addressed wrongly does not stop at any budget, which is the
chat-template probe's business (F38, A7).

**The value applied is the longest turn observed. Not an average, not a
margin.** An average truncates half the answers; a margin is a number MCF
invented, and §3.15 has no room for one. What is claimed is exactly what was
measured — *this many tokens were enough for every turn that finished here*.

**A caller who says nothing is not a caller who says the default.** The budget
only applies where the caller gave none, and making that true required the
distinction to exist on the wire: `mcf run` substituted its default *before*
sending, so the daemon could not tell `--limit 32` from silence. The request
now carries `None`, and the daemon resolves the caller's word, then what
somebody derived, then MCF's stated default. Verified both ways: silence gets
313 and finishes at 65, `--limit 8` gets 8 and is cut off. D43 is that MCF
never changes a value under somebody who set it, and that rule is unenforceable
if the value arrives already substituted.

**Each derived parameter carries its own provenance.** The addressing and the
budget are set by different probes on different days through possibly different
engines, and a budget citing the chat-template probe would be a value citing an
experiment that did not measure it (B-059). The file holds them separately, and
writing one reads the file first so that setting a budget cannot silently drop
an addressing (A1).

**A gate test that depends on whether a daemon happens to be running** (fixed
in F47).
`run::tests::a_file_that_is_not_a_model_is_refused_legibly` asserts the refusal
says *no vendored engine* — true when MCF answers for itself, false when a
daemon is up and a provisioned llama.cpp produces its own refusal instead. It
failed during this work because a daemon was left running from a manual check,
and passed when it was stopped. **The test is right and the isolation is
missing**: §3.12 makes a suite whose result depends on ambient state a suite
that cannot be reproduced, and this one silently reports on whichever machine
state it found. Registered as B-378 rather than fixed here, because the fix is
about how the CLI tests reach a daemon and is not this probe's business.

**What was not established.** Whether the longest turn observed over five short
factual questions is the longest turn this model has — it plainly is not, and
the probe claims only what it measured. A model asked to write an essay will
exceed it, and the honest reading of the applied budget is *enough for turns
like the ones asked*, which is why the questions are part of the method. Nothing
about a model that stops at wildly different lengths depending on the question:
10 to 313 on this one, and a single number for a distribution that wide is a
choice this probe makes and states rather than one it justifies. And no timing —
sixteen trials and 1952 tokens is a cost, not a speed (B49, B35).

## 47 · F47 — The suite was reporting on the machine it found (B-378, B-003, B16, §3.12, F46)

**What was wrong.** `run` looked up whether a daemon was listening *in the
middle of doing its job*, so a test asserting on the refusal MCF gives for an
unreadable file was really asserting on whichever of MCF and a daemon answered.
It passed alone and failed beside a running daemon, for reasons nothing in the
test could see. F46 found it by leaving one running.

**Why it matters more than one test.** This suite has been the arbiter of every
change in this document. A gate whose answer depends on the state of the
machine it ran on is not a gate — it is a different experiment each time,
reported as though it were the same one, and §3.12 is exactly about the
difference.

**The fix is that where the daemon is becomes an input.** `run_where` takes it;
`run` looks it up and passes it in. `None` means *no daemon*, which is both what
a machine with no runtime directory gives and what a test wants to say. The
tests call a helper named `without_a_daemon`, so what they assume is in the
name rather than in the environment.

**Shown to work in the direction that matters.** The old failure mode was *pass
without a daemon, fail with one*. The tests now pass **identically with a
daemon running and with none** — which is the assertion, and running it only
one way would have demonstrated nothing (B-003).

**A check, so it cannot come back.** A small table of entry points that reach
for ambient state, each with the sibling that takes it as an argument, and the
files watched for calls to them. Shown to fire: putting the ambient call back
produced

```
crates/mcf-cli/src/run/tests.rs:47: calls `run(` — use `run_where(` instead,
because it looks up whether a daemon is listening…
```

A second test asserts that every sibling the table names **exists**, because a
check that tells somebody to call a function that is not there fails the reader
rather than the code.

**The table is small on purpose, and that is a limitation rather than a
design.** It watches one call in one file. Nothing stops a *new* function from
acquiring the same shape unnoticed, and nothing checks the other ambient
readers — the model store, the record's path — which are read by tests that
mostly pass their own scratch directories and were not audited here. What is
mechanical is the regression, not the class.

**What was not established.** Whether other tiers have the same dependency. The
laboratory's serving scenarios build their own socket under a scratch world and
`mcf-serve`'s cost tests do the same, which is why only the CLI's was found —
but *these three read ambient state and are fine* is a survey of three, not of
the suite.

## 48 · F48 — The tie was MCF's, not the model's: a template that names a role in order to rename it (B-375, B-376, D42, D46, §3.7, F38, F39, F40)

**The question B-375 asked.** gemma-3-270m had two addressings that both
answered and ended the turn five times of five, so *did the turn end* could not
separate them. F38 offered turn length as the sharper question and F40
withdrew it — it separated them on one engine and not on another, which made it
a reading about the instrument. What was left was to find a question that
holds.

**There was no question to find, because there was no ambiguity.** The two
candidates differed only in the role word — `assistant` against `model` — and
gemma's own template says which:

```
{%- if (message['role'] == 'assistant') -%}
    {%- set role = "model" -%}
```

It names `assistant` **exactly once, and does it to rename it**. MCF read the
template as a bag of words: *does this text contain "assistant"? then that is a
candidate.* So it manufactured a candidate the file explicitly rejects, failed
to tell it from the real one, and reported the file as ambiguous when the file
is explicit. Two findings' worth of searching for a sharper observation, and
the defect was in the question's premise.

**Mentioning is not meaning.** A word a template *compares against* is an input
name on its way to being translated; a word it *assigns* is what gets written
out. MCF now reads the assignment — `set <name> = "literal"`, both quotings,
every occurrence — and falls back to the mentioned names only when a template
assigns nothing, which is the case where it emits the role it was given and the
mentioned names really are the candidates.

**It reads the template's shape and does not execute it.** A chat template is a
program in somebody else's language and running one is a door §3.7 keeps shut —
which is also the position taken when this was last raised. What is recognised
is one shape, the one that matters, and nothing else; that is the honest extent
of reading a program without running it, and it is stated rather than implied.

```
gemma-3-270m   start_of_turn…end_of_turn as model   5 of 5 ←
               raw                                  4 of 5
               best: as model — agrees with the file
```

**A behavioural question was tried first, and is worth recording because it
failed on the model it was for.** Stop the turn *before* the role word and let
the model supply it: `<start_of_turn>user\n…<end_of_turn>\n<start_of_turn>` and
one token. SmolLM2 answers `ass` — the first token of `assistant`, its own role
word, exactly right. gemma-3-270m answers a **newline**. The question is sharp,
needs no judgement, and does not discriminate on the one model with a tie. It
is not built.

**And the fix uncovered a plumbing defect it would have hidden.** With gemma
deciding, the stop-condition probe ran on an *unconfigured* model for the first
time and returned inconclusive: with nothing applied it sent the question as
**text**, which routes to the engine that takes a command line and cannot say
why it stopped (B-376). Every unconfigured model — most of them — would have
reported inconclusive for a reason that is MCF's plumbing rather than the
model's behaviour. The question travels as identifiers now whether or not it is
wrapped.

**What was not established.** How many real templates the assignment rule
reads correctly: two, here. A template that builds the role by concatenation,
or in a macro, or with a variable it assigns twice, is not handled and is not
claimed to be — a template naming two roles yields two candidates, which is a
tie MCF has evidence for, unlike the one it invented. And nothing here verifies
that `model` is *right* for gemma beyond the model ending its turns under it:
that is the same observation as before, on a candidate set that is no longer
wrong.

## 49 · F49 — MCF can check its own engine against the one it built, on a user's machine (B-362, B-376, D31, D39, A12, A19, §II)

**What was built.** `mcf cross-check <model>` — the two engines a *user* has,
compared on the same input, on their machine, about their model.

**Why it is not the oracle.** `scripts/check-oracle.sh` compares MCF against a
reference implementation and needs a checkout of somebody else's source; it
runs when this repository is being changed. This asks the same question of
MCF's own engine and the one **MCF built into a prefix** (B-367, D39) — which
is a thing a user has. §II is why it exists: MCF makes claims about models, and
a claim computed by an engine nobody has checked is a claim about the engine.
A12 says MCF may not ask to be trusted, and this is the shape of not asking.

**It was unbuildable until B-376.** Comparing two engines requires giving both
the same input in the same form and getting comparable output back. A
completion tool takes a command line and prints text; the server takes
identifiers and hands them back. The whole comparison rests on the second.

**Teacher forcing, because texts cannot be compared.** Two correct
implementations agree until two tokens are close enough that summation order
picks a different winner, and after that they are writing different sentences
(F27, F40). MCF reads the *other* engine's tokens and at each position is asked
what it would have chosen.

**The rank, not the margin.** F40 measured both rules and F41 measured the gap.
The margin threshold was calibrated on one parting step per file and raises
false alarms at every position of a long comparison; the rank separates by two
orders of magnitude. The line is **8**, and this run confirms where the clean
side of it sits:

| | agreed | worst rank |
|---|---|---|
| SmolLM2-135M Q8_0 | 116 of 120 | 2 |
| gemma-3-270m Q6_K | 111 of 120 | 3 |
| TinyMixtral-4x248M Q5_K_M | 117 of 120 | 2 |
| Qwen3-0.6B Q4_K_M | 113 of 120 | 2 |
| SmolLM2-135M **Q2_K** | 117 of 120 | 1 |

Four architectures including a mixture of experts, and the quantization F33
found the widest arithmetic gap on. Every one within rank 3 of a line at 8.

**Shown to fail, and the first mutation was again the wrong one.** Swapping the
rotation's pairing changed nothing — because the arm edited was one llama never
takes; the family falls through to the default. That is F41's lesson arriving
again in the same session, and it is worth writing down twice: **a negative
control that does not touch the subject demonstrates nothing, and looks
identical to one that does.** Mutating the arm the family *does* take:

```
clean               agreed 116 of 120   worst rank    2   AGREE     exit 0
rotation swapped    agreed  37 of 120   worst rank 3388   DIVERGE   exit 1
```

**What it refuses to say.** Not *which* engine is wrong. Neither is the
authority — what is compared is two readings of one file, and a disagreement is
a finding about one of them. Saying which would need a third reading, and
claiming it from two would be exactly the manufactured certainty A19 forbids.

**A precondition checked before a model is loaded.** *Nothing to compare
against* is a fact about the other engine's answer and needs no model at all.
The first version parsed the file first and a laboratory scenario caught it —
the same shape as F44, where a probe paid eight thousand forward passes to
learn its instrument could not answer.

**What the gate caught.** `probe.inconclusive` claimed with no scenario able to
produce it (A13), and then a failure carrying no context. Both are checks that
have now fired on four consecutive pieces of work, which is either a very good
suite or a very consistent author.

**What was not established.** Whether 120 positions is enough: F40 used 700 and
found agreement improving with position, so a short comparison is the
conservative direction, but *enough* is not measured. Nothing about a model too
large for MCF's own engine — the reference model cannot be cross-checked at
all, because MCF cannot read it (B-372), which is precisely the case where a
user would most want the check. And the threshold is provisional in the
direction all of them here are: a defect that only ever swaps the top two
tokens passes.

## 50 · F50 — The privileged helper could be told where the machine is (B-190, D35, §6.32, §XVII, A2, A26)

**Found by preparing to use it.** D35 names three privileged operations and one
of them is *read the processor's energy counter* — the awkward one, a read that
needs elevation, without which there is no energy-per-token figure on a
processor at all. Energy was raised as a fourth axis for the first benchmark,
and the question was whether this machine could measure it. It can, and the
helper that would do it was already built and already correct in every respect
this document had checked.

**It accepted `--under`.** The flag rebases every fixed path, and it exists so
that three laboratory scenarios can drive a privileged program against a
fixture without letting it near the machine (D26). It was parsed by the
**shipped binary**, not only by the tests. Demonstrated as an ordinary user,
against a directory made a moment earlier:

```
$ mcf-helper energy --under /tmp/…/fake
name: 4242
```

**With no privilege that is harmless, which is exactly why it survived.** The
helper has never held any. But this program exists *in order to* be given some,
and the moment it is — by a capability, by setuid, by a line in a sudoers
file — `--under` is a hole the size of the privilege:

- `energy --under <a tree you control>` reads any file the privilege can reach,
  through a symlink at the path it expects.
- the governor operation *writes* under the same rebased root, and the value it
  writes is chosen from `scaling_available_governors` — which is read from
  under that root too, so the caller supplies that as well.

A local privilege escalation, latent, waiting for the grant that was about to
be made. §6.32 is the section that asks how a privileged daemon avoids becoming
a way to run anything as root, and the answer it settled on — per-operation,
minimal, auditable, a separate executable that exits — was implemented
faithfully. The hole was in an argument nobody classed as part of that surface.

**The fix is that the seam is not in the program.** `run` takes no root and
refuses `--under`; `run_under(root, arguments)` is a parameter, reachable from
the laboratory and this crate's tests and from nothing else. Refused rather
than ignored: a caller who asked for something and did not get it must be told
(A2), and silently reading the real machine instead would be worse than either.

**One existing test asserted the weaker guarantee.** `under_cannot_name_a_file_to_write`
checked that a root naming a *file* produced a platform failure and left the
file untouched — true, and it was checking that the rebasing was survivable
rather than that it should not exist. It now asserts both: the shipped program
refuses the argument, and the parameterised form still declines a root with no
processors under it.

**What this says about the shape of the audit.** The repository has a check
that nothing shipped reaches for elevation and that nothing links the helper
but the laboratory, and it passes — the danger was never that the daemon would
become privileged. It was that the helper's *input* was wider than its
operations, and a surface enumerated as three operations was really three
operations and a root. **An enumerated surface is only enumerated if the
arguments are part of the enumeration.**

**What was not established.** Whether the other two operations have a
comparable widening — the accelerator one takes an index and shells out to a
vendor tool, which is a second thing to look at with the same eyes and has not
been. Whether any other program in this repository takes a test seam through
its shipped argument parsing: one was found by needing it, not by looking.
And the grant itself has still not been made, so nothing here is a claim that
energy is measurable — only that the program which would measure it is no
longer a way to read anything else.

## 51 · F51 — Contention moves the level, not the spread, and that decides how a benchmark must be built (DEC-007, B-250, B-181, §3.4, A19, F2, F3)

**What was measured.** The first thing the operator's answer on DEC-007 asked
for: not a chosen repeat count but a measured one. One deterministic run —
MCF's own engine, eight tokens, greedy from a fixed seed, so that only the
timing varies — repeated on this machine as it is, and again under sixteen
deliberate burners. `prototypes/timing-noise` is the instrument.

| | machine as it is | under sixteen burners |
|---|---|---|
| load average during | 9.8 – 10.8 | 10.5 – 22.5 |
| median | **3.202 s** | **5.322 s** |
| middle half of runs spans | 4.2% of the median | 9.1% |
| slowest ÷ fastest | 1.11 | 1.16 |

**The finding is the shape of the difference, not its size.** Contention made
the run **66% slower** and made the spread **twice as wide**. Those are not
comparable magnitudes. A busy machine does not mainly add noise — it moves the
whole distribution, coherently, and keeps it nearly as tight as before.

**Which decides how a comparison must be built, and it is not by repeating
more.** If two configurations are measured one after the other and the
machine's load changes in between, the error is the *level shift* — up to 66%
here — and no repeat count removes it, because every repeat of the second
configuration is wrong in the same direction. If the two are interleaved, a
level shift lands on both arms equally and cancels. B-250 already asks for
paired, interleaved, order-randomized trials as a principle; this is the number
behind it, and it says the principle matters roughly **fifteen times more** than
the repeat count does.

**How many repeats, derived.** The samples were resampled against themselves —
two groups drawn from the *same* measured timings, four thousand times — and
each candidate count asked how often two such groups differ by more than the
effect being looked for. Every difference found that way is noise pretending to
be one, since there is no real difference by construction. No assumption is
made about the distribution's shape, which matters because a wall time has a
floor at the work itself and a tail made of whatever else the machine did.

| to detect | on this machine as it is | under load |
|---|---|---|
| 2% | 50 repeats | more than 100 — the noise is larger than the effect |
| 5% | **7 repeats** | 50 |
| 10% | 3 | 7 |
| 20% | 3 | 3 |

**Seven repeats is the answer for a five-percent claim on this machine in its
normal state**, and 2% is not honestly reachable at any repeat count a person
will wait for. That is a fact about this machine and this workload, not a
constant — which is why the instrument travels rather than the number.

**It also settles the argument it was built to settle.** The as-is column was
taken at a load average of ten on sixteen cores — the machine was not quiet,
and the numbers are perfectly usable. An absolute quiet threshold would have
refused that measurement and would have been wrong. What the baseline actually
consists of here is other projects' tooling: language servers nine hours old,
and another project's process. **It is not going away**, so a rule that waits
for it to go away never fires. The operator's criterion — stability against the
machine's own baseline — is the one the data supports.

**What was not established.** One workload, one model, one machine, one engine —
MCF's own, which is not the engine benchmarks will use; the provisioned server
is faster and its noise has not been measured, and a shorter run may well be
noisier in relative terms. Wall time only: no energy, no memory, no
thermal state, and the processor's frequency governor was left as it was, so
part of the 4.2% may be frequency scaling that pinning would remove. Nothing
here is a *speed* — B65 forbids one from MCF's own engine, and these are
durations under stated conditions used to characterize the *machine*, which is
what the engine is a fixed load for.

## 52 · F52 — The engine benchmarks will use is three times noisier than the one they will not, and two guesses about why were both wrong (DEC-007, B-366, B-376, §3.4, A19, F51)

**Why this was measured.** F51 derived a repeat count — seven, for a five
percent claim — from MCF's *own* engine, and said plainly that this was the
wrong engine: benchmarks will use the provisioned one. This is that gap closed,
and the answer changes the number.

**Every figure below was taken with the machine holding still.** The instrument
now asks whether a run's duration tracked the machine's load, which is the
operator's stability criterion put in the one form that needs no threshold. Any
measurement where it did is reported as being *of a machine that changed*
rather than of the command.

| engine | path | run | middle half | 5% needs | 10% needs |
|---|---|---|---|---|---|
| MCF's own | in-process | 3.2 s | **3.6%** | 5 | 3 |
| MCF's own | through the daemon | 4.9 s | **3.6%** | 7 | 3 |
| provisioned | through the daemon | 0.23 s | 13.7% | 50 | 15 |
| provisioned | through the daemon | 2.0 s | **9.9%** | 50 | 10 |
| provisioned, **one thread** | through the daemon | 4.8 s | **45.8%** | >100 | >100 |

**The engine that will be measured is about three times noisier than the one
that will not.** Fifty repeats for a five percent claim, against seven. That is
the number DEC-007 actually needs, and F51's seven was an answer about the
wrong subject.

**Two guesses, both wrong, both caught by measuring.**

*The path.* The obvious suspect was the daemon and its socket — a round trip
and a subprocess between the timer and the work. It contributes **nothing**:
MCF's engine measures 3.6% in-process and 3.6% through the daemon. Ruled out.

*The threads.* The next guess was that llama.cpp is multi-threaded and MCF's
engine is not, and that a process wanting sixteen cores on a machine already at
load ten finishes when it is given them. Pinning the server to one thread
should then have tightened it. **It made it five times worse** — 45.8% against
9.9% — on a clean measurement. The prediction was exactly backwards and the
mechanism is not established.

**So thread count is a condition, not a detail.** It moves benchmark noise by a
factor of five, in a direction that is not obvious from reasoning, and a timing
that does not record it is a timing nobody can reproduce (§3.4). This also
bears on B-366: giving MCF's own engine threads will change its noise
characteristics, and the change must be re-measured rather than predicted —
this finding is what says predicting it does not work.

**A measurement of mine was contaminated and I did not notice until the
instrument was taught to.** An earlier reading of the same command reported a
middle half spanning 121.6% and nothing detectable at any effect size. The load
average had gone from 9.9 to 19.6 *during* it. That is precisely the condition
the operator's criterion invalidates, and the instrument reported the range and
drew no conclusion from it — leaving the conclusion to me, which is the wrong
division of labour. It now computes whether duration tracked load and says so
in a sentence nobody can read past.

**What was not established.** Why the provisioned engine is noisier, which is
now an open question rather than an answered one — two mechanisms were tested
and neither holds. Whether the one-thread result generalises or is particular
to this model and this machine. Anything about energy, memory or thermal state.
And whether pinning the frequency governor narrows any of these, which is the
last of DEC-007's open pieces and needs the privileged helper that F50 has only
just made safe to grant.

## 53 · F53 — The noise floor is a property of the moment, not of the machine (DEC-007, B-083, B-181, D35, A19, F51, F52)

**What was asked.** The last of DEC-007's open pieces: whether pinning the
processor's frequency would narrow the noise F51 and F52 measured. It was
expected to need the privileged helper F50 has just made safe to grant. It did
not.

**The governor grant buys nothing here, and that was answerable without any
privilege.** This machine's governor is already `performance`, and the only
other one it offers is `powersave`. There is nothing to pin. D35 lists setting
the governor as one of three privileged operations, and it remains right to
have — on a machine that is *not* already at performance it would matter — but
on this one the answer is that the reading which decides it costs nothing.

**Frequency moves anyway, and a governor cannot stop it.** Boost is on, the
range is **0.62 to 5.76 GHz**, and at any instant the cores are spread across
most of it — idle ones at 0.62 while busy ones sit at 5.5. Across runs the mean
moved 4.26 to 5.36 GHz. So a run's clock depends on which core it lands on and
what else is running, and no governor setting removes that.

**But frequency does not explain the noise, and neither does load.** The
correlations between duration and each of them, across five clean rounds of the
same command: **−0.36, +0.09, +0.56, +0.15, −0.17** for frequency and
**+0.32, +0.21, −0.07, −0.42, −0.18** for load. They change sign between
rounds. A permutation test says why: at twenty samples, a coefficient of
±0.15 is what chance produces **48% of the time**, and ±0.42 about **5%**.
None of these is evidence of a mechanism. **F52's question stays open**, and
this closes two candidates rather than answering it.

**The finding is what happened while looking.** Six clean measurements of the
*same command, the same engine, the same machine*:

| middle half spans | 2.6% | 7.4% | 7.4% | 7.9% | 8.5% | 9.9% |
|---|---|---|---|---|---|---|
| repeats for a 5% claim | 7 | 20 | 30 | >100 | 50 | 50 |

**The noise floor is not a property of the machine. It is a property of the
half-hour.** A number derived from one sitting and written into a document is a
number about that sitting. F51 published seven, F52 published fifty, and both
were honest reports of what was in front of them.

**Which changes what an acceptance criterion can be.** *At least N repeats*
cannot be the rule, because N is not stable — it varied sevenfold here with
nothing changed but the time of day. The rule has to be a **stopping condition
rather than a count**: a benchmark repeats until its *own* resampling says the
difference it is looking at is bigger than the noise it is measuring, and
reports how many that took. That is the same principle the operator already
set — measure, do not choose — applied one level deeper than it was meant, to
the measurement of the measurement.

**And my own instrument had a chosen number in it.** Contamination was called
at a correlation of 0.5, picked because it sounded like a lot. At twenty
samples it is barely above what chance produces, so it would have flagged clean
runs and missed dirty ones. It now shuffles the pairing four thousand times and
reports how often chance alone is that tight — no threshold, and the reader
sees the same figure the decision is made on.

**What was not established.** Why the provisioned engine is noisier than MCF's
own; two more candidates are eliminated and the question is still open.
Thermal state — the only sensor this machine exposes reads sixteen degrees,
which is not a processor temperature, so thermal steady state cannot currently
be observed at all here and is the one piece of DEC-007 that stays genuinely
unanswerable. And whether the sevenfold variation in the noise floor narrows
inside an exclusive window, which is the argument *for* the window and has not
been tested because the window is not built.

## 54 · F54 — The stopping condition, and the first comparison that stopped itself (B-083, B-086, B-250, DEC-007, F51, F52, F53)

**What was built.** `mcf_bench::enough` — the rule a comparison carries instead
of a repeat count. F53 established that a count cannot be the rule, because the
same command on the same machine needed seven repeats in one sitting and over a
hundred in another. So a comparison repeats until *its own* resampling
separates the difference it is looking at from the noise it is measuring, and
reports how many that took. **The count becomes part of the result rather than
part of the policy.**

**Three outcomes, because two would be a lie.** *They differ*, *they are the
same to within a stated resolution*, and *not yet decided*. The middle one is a
real answer — B-086 says a null result is a result — and it carries the size of
the difference that would have shown, so *no difference* never means *we
stopped looking*. The third is D42's honesty applied to a comparison: arms that
have not separated, with noise still wider than the question, must not be
rounded to either answer.

**Paired by construction, because the pairing is the defence.** F51 measured
contention moving a run's whole distribution by sixty-six percent while
widening it only from four to nine, so *all of A then all of B* carries any
drift as an error landing on every repeat in one direction. Arms of unequal
length are refused rather than truncated: a caller who has run one arm more
than the other has not run a paired trial, and truncating would silently
produce the shape B-250 exists to prevent.

**The first comparison that stopped itself**, on the question the first
benchmark is actually for — two quantizations of one model, 150 tokens each,
alternating. Twice, an hour apart:

```
not decided after 10 paired trial(s)
they differ by 9.6%, after 17 paired trial(s) — noise alone produced a gap
that big 4.5% of the time      · medians 1.130 s and 1.031 s

they differ by 36.2%, after 8 paired trial(s) — noise alone produced a gap
that big 3.9% of the time      · medians 1.129 s and 0.829 s
```

Seventeen, then eight. Not the seven F51 derived, nor the fifty F52 derived —
**the number was found by the run rather than brought to it**, which is the
whole of what F53 asked for. And the two sittings disagree about the *size* of
the difference — 9.6% against 36.2%, from a slower second arm rather than a
faster first — which is F53's point arriving again: what a single sitting
measures is what that sitting had.

**A shipped crate here may not hold a floating-point number**, because that is
how a NaN reaches a record — a rule this module broke on its first draft and
that the gate caught. It counts in nanoseconds and parts per million now, which
is not merely compliance: a ratio of integers cannot be a NaN, a zero duration
is *not yet decided* rather than an infinity, and the existing measurement
vocabulary already had the type for it.

**What was not established.** One machine, one pair of files, one prompt
length, and two sittings that disagree by a factor of four about the size of
the gap: that Q2_K is faster than Q8_0 here is a demonstration that the
instrument works, not a finding about quantization — the frontier is B-091 and
this is not it. The ceiling of a hundred and twenty paired trials is a chosen
number, as is the one-in-twenty false-alarm rate; both are stated in one line
each. And the resampling seed is fixed, so re-asking the same data gives the
same verdict — which is reproducibility (§3.12) and not certainty, and the two
are easy to confuse when a number comes back identical twice.

## 55 · F55 — The pairing is worth eighty-eight percent, and the stopping condition could only answer one way (B-250, B-083, B53, §3.27, DEC-007, F51, F53, F54)

**What was asked.** B-250's condition is that *block-then-subtract does not
compile*. F51 had measured why it should not — contention moves a run's whole
distribution rather than widening it — but the number that mattered was never
taken directly: **how large a difference does the blocked arrangement invent
between two things that are identical?** And whatever the answer, the type had
to stop it being expressible.

**The blocked arrangement invents eighty-eight percent.** One command,
`mcf run` over a real model file, two hundred tokens, run twenty times and then
twenty times again. Nothing about the command changed between the two blocks;
forty-eight busy loops started on this thirty-two-core machine did.

```
arm A, quiet          median 0.233 s   middle half spans  4.4%
arm B, under load     median 0.439 s   middle half spans 43.2%
```

Subtract those two blocks and MCF would report **one configuration 88% slower
than an identical one**, with a spread that looks like evidence rather than
like a warning. That is the failure B53 calls *thirty runs of A, then thirty of
B, subtracted*, priced on this machine.

**Interleaved, the same machine change costs nothing.** The paired runner —
alternating the arms, drawing which goes first per pair — was asked the same
null question while sixteen burners arrived a third of the way through. The
load episode is visible in the raw trials, four pairs at 0.24–0.27 s among
thirty at 0.15 s, and the verdict is unmoved:

```
no difference as large as 2.0%, after 30 paired trial(s) — one that big would
have shown          · medians 0.149 s and 0.149 s
order: 16 pair(s) ran the left arm first, 14 the right
  …
  #19: 0.169 s  0.145 s  right first  right quicker by 16.5%
  #20: 0.242 s  0.257 s  left first   left quicker by  6.4%
  #21: 0.267 s  0.256 s  left first   right quicker by 4.0%
  #22: 0.193 s  0.205 s  left first   left quicker by  6.0%
  #23: 0.172 s  0.158 s  left first   right quicker by 8.3%
  #24: 0.148 s  0.149 s  right first  left quicker by  1.0%
```

Pairs #20 to #23 are seventy percent above the rest of the run and contribute
nothing to the answer, because the load landed on *both* arms of each of them.
That is the whole of §3.27 in six lines of a real run.

**And the mechanism is provable exactly, without a machine.** A test on the
simulated clock builds forty-eight timings — a level that steps up halfway
through, a wobble belonging to the round, a jitter belonging to the run — and
arranges *the same forty-eight numbers* two ways. Interleaved: *no difference
as large as 20.0%*. Blocked: *they differ by 50.0%, noise alone produced a gap
that big 0.0% of the time*. Only the arrangement differs, and it is the whole
of the answer. A fixture whose jitter cycled with period six failed this first
— its noise was confounded with the even/odd positions the pairing uses, and
the paired comparison correctly found the three-percent difference the fixture
had accidentally built. The fixture was wrong, and the instrument said so.

**What was built.** `mcf_bench::compare`. A `Comparison` has three
constructors and there is no fourth: `Interleaving`, which runs the arms
alternately itself and draws the order per pair from a stated seed;
`from_trials`, which reads a session back out of the record and **verifies the
interleaving from the positions** — the merged trials are taken two at a time
and every couple must hold one of each arm, so blocked trials come back as
`RanInBlocks` naming the arm and where it repeated; and
`from_separate_sessions`, which is §3.27's *it may be all that exists*.

The statistics moved behind that door. `enough`'s two entry points are
`pub(super)`, because a function taking two slices of timings cannot tell an
interleaved comparison from two blocks and never could — that is B-250's *does
not compile*, and `checks/tests/a_comparison_is_paired.rs` is what keeps it
true when somebody adds a fourth way in.

**The weaker claim is weaker in the type, not in a label.** A comparison
assembled from separate sessions answers `None` to `paired_differences()`:
there is no pairing, so there is no paired difference, and a difference of two
summaries cannot be handed out wearing this one's name. The label exists as
well, in every rendering. The reverse is refused too — two arms that turn out
to share a session may not be assembled the weak way, because the strong
construction is available and choosing the weak one would be discarding
evidence.

**The reported quantity changed, and so did the null.** F54 pooled both arms
and redrew independent groups, which is an unpaired test on paired data: it
throws away the very structure that makes the comparison durable. The statistic
is now the median of the *paired differences* and the null is a sign flip —
under *these arms are the same*, which arm came out ahead in a given pair is a
coin toss. That change also exposed a defect the pooled version hid.

**The defect: the stopping condition could only ever answer one way.** The
first real run of the new instrument, one command against itself, stopped after
**four paired trials** and declared *no difference as large as five percent* —
from differences of 12.9%, 7.4%, 0.2% and 1.4%. It cannot be. A sign flip over
four pairs has sixteen assignments, so the smallest false-alarm rate reachable
is one in sixteen, above the one in twenty the module requires: `Differ` was
**unreachable at that count whatever the data**, and a rule that can only
answer one way is not a test.

The cause was a circular question. *Would a five-percent difference have
shown?* was being asked of a null built out of the very differences a
five-percent difference would have moved. It is now asked as power: centre this
run's differences on their own median, add an effect of exactly the resolution
asked for, and ask whether *that* would have been declared. On the same four
real differences the answer is *not yet*; on noise of the same size, centred, a
five-percent null result arrives at **eighty pairs** — not four.

The same correction went into the pooled path, which asks it by scaling one arm
rather than shifting a difference. Both are `Same`-branch questions only: the
`Differ` branch was never circular.

**What was not established.** The eighty-eight percent is one machine, one
command, one load pattern; a different machine and a lighter load give a
smaller number, and the point is the *shape* rather than the size — F51 already
showed the shift is coherent rather than random, and this shows what
subtracting two blocks does with it. The machine also hosts other projects'
workloads, so the *quiet* arm is quiet relative to its own baseline (§3.4,
DEC-007) rather than absolutely. The order-randomization generator is a
xorshift with a stated seed, which is reproducibility and not
unpredictability — nothing here needs the latter. And the eighty pairs above is
this sitting's number: F53's finding stands, and the count is still a property
of the half-hour.

## 56 · F56 — A confounded comparison has no delta to give (A8, B-085, §3.4, A7, F55)

**What was asked.** A8 says *a comparison is only meaningful when one thing
differs; when more than one did, the honest output is "these are not
comparable", not a delta.* F55 made a comparison a type that knows how it was
built. This makes it a type that knows what it is a comparison **of** — and
refuses to answer when it is a comparison of nothing in particular.

**An arm is a configuration, not a name.** `UnderTest` carries an `Arm` and the
`Conditions` it is measured under, and every constructor of a `Comparison`
takes two of them. An arm that carried only a name would leave the question
unanswerable, and a comparison that cannot say what it isolated is one whose
delta a reader will over-read.

**Four answers, computed from the floor itself.** `mcf_core::measurement::Isolation`
walks `Floor::entries()` rather than a list of field names written out again,
which matters more than it looks: §3.3 says the floor never shrinks and does
not say it never grows, and a second copy of the list would be the thing that
quietly stopped growing with it — the comparison would go on reporting
*isolated* about a condition it had stopped looking at. A check enforces that
the isolation module names no condition of its own.

- *Same configuration.* Nothing differs. Not a failure and not a confound: two
  arms of one configuration measure the machine's own noise, which is the
  control every comparison should be able to run against itself — and is
  exactly the null run F55 used.
- *Isolated.* Exactly one condition differs, and it is named.
- *Confounded.* More than one differs. **The verdict is `None`.**
- *Undetermined.* A condition is `Unknown` on one side or the other.

**Two unknowns are not a match**, which is A7 pointed at a comparison. *They
were probably the same* is a plausible value substituted for something MCF did
not read, and it is the substitution that would make this whole check
worthless: today almost every condition producer is unbuilt, so a rule that
counted unknown-against-unknown as agreement would report *isolated* about
every comparison MCF can currently take.

**Ordering, stated because it is a judgement.** Two known differences are a
confound whatever else is unread. A comparison that has already lost its
meaning does not recover it by MCF failing to read a twelfth condition, and
reporting *undetermined* there would be the softer of two answers where A8
wants the harder one.

**The refusal is in the type.** `Finding::verdict()` returns
`Option<&Verdict>`, and it is `None` exactly when the arms are confounded and
the operator has not declared it. Not a flag beside the number and not a
warning in the rendering — there is no number, so a caller cannot print one by
forgetting to ask. On twelve interleaved trials in which one arm takes twice as
long, with quantization *and* thermal state differing, the rendering is *these
are not comparable: 2 conditions differ (thermal\_state, quantization)* and the
words *they differ by* appear nowhere in it.

**A confound the operator declares is science** (A8), so `declaring(because)`
exists and returns the delta with the declaration and every differing variable
printed beside it. MCF does not judge the declaration and cannot: whether two
variables may honestly move together is a statement about the question being
asked, not about the machine. What it will not do is let the delta out without
them.

**What this immediately says about MCF's own instrument.** The timing prototype
compares two shell commands. It can state one condition — the command line, as
`mcf_configuration` — and cannot state the other ten, so its findings now read:

```
they differ by 76.4%, after 10 paired trial(s) … — isolation is undetermined:
mcf_configuration differs, and 10 condition(s) could not be compared
(hardware_state, thermal_state, driver_versions, runtime_versions,
quantization, context_length, batch_shape, realized_placement,
instrumentation, artifact_storage)
```

That sentence is longer than the number it qualifies, and it is the honest
description of what a prototype timing two opaque commands has established.
Every one of those ten is a producer that does not exist yet (B-007, B-013),
so the sentence shortens as MCF learns to read its own machine — which is the
point of asking the question in a form that can be answered later.

**What was not established.** The comparison is of *values*, so two conditions
recorded in different words — `q8_0` against `Q8_0` — read as a difference.
Normalizing them would be MCF deciding two strings mean the same thing, which
is a judgement it has no basis for; the producers that will write these fields
are what makes the values comparable, and until they exist the strings are the
caller's to keep consistent. Nothing here checks that a *declared* confound is
a reasonable one, and nothing can.

## 57 · F57 — The sign test, and the resampling that could not see identical arms (B-086, B-083, DEC-007, A9, F51, F55)

**What was found, and how.** B-086 needed a test that records a null result, so
the first fixture written for it compared two arms with *identical* timings —
the clearest null there is. The stopping condition answered *not decided*, at
forty pairs, about data that could not be clearer.

The cause is in the statistic. F55's null was a sign flip over the paired
differences, judged by the median's size. When every difference is the same
magnitude — and *identical arms* is the extreme case, every difference exactly
zero — flipping signs cannot move that size. The null distribution is a single
point, the observed value sits on it, and the p-value is one, for ever. **An
instrument that cannot recognize the cleanest possible null is not a
conservative instrument; it is a broken one.**

**What replaced it: the sign test, exact.** Under *these two arms are the
same*, which arm came out ahead in a given pair is a coin toss. The chance of a
split as lopsided as the one observed is a sum of binomial coefficients — whole
numbers, computed in `u128`, with no resampling, no seed, and no approximation.
Ten pairs won by one arm is 1,953 parts per million; a test asserts that exact
figure, which a resampling could only have come near and would have moved
whenever the seed did.

It also uses **only the ordering of two values within a pair**, which is what
`Quantity` guarantees and all it guarantees. F51 measured a tail made of
whatever else the machine was doing; a statistic that weighs by magnitude
carries that tail into the answer.

**The cost is stated, and it arrived immediately.** Discarding magnitudes
discards power when the noise is well behaved, and the test will call a
difference real that is far too small to act on. The very first fixture written
after the change gave the left arm a systematic extra millisecond in a run of
one-second trials — forty pairs out of forty, a real difference of **a tenth of
a percent**, found and reported. The fixture was wrong and the instrument said
so on its first use, which is the best possible demonstration of both halves of
the trade.

**Two properties the exact form makes checkable.** Six pairs is the fewest that
can reach one in twenty at all — five gives one chance in sixteen — so `Differ`
is arithmetically unreachable below six, whatever the data. That is F55's
four-pair defect in its general form, and it is now a test rather than an
anecdote. And `2^n` leaves `u128` past a hundred and twenty-seven pairs: the
first draft answered *not decided* at a hundred and sixty pairs of data it had
decided at eighty, which is **an instrument whose confidence falls as its
evidence grows**. The counts are now scaled down to the largest exactly
computable size with the winner's count rounded down, which can only weaken the
claim, never strengthen it.

**What it costs and buys, on this machine.** The same noise that needed eighty
pairs under the resampling test reaches a five-percent null result at **twenty**
under the sign test. A real difference between two token budgets, previously
found at ten paired trials, is now found at **six** — 88.1%, the minimum count
the test admits. Two clean runs of one command against itself established *no
difference as large as 5%* at twelve and at twenty-eight pairs, which is F53
arriving again: the count is a property of the sitting.

**What was not established.** The sign test's power depends on the noise being
symmetric about zero under the null, which pairing makes reasonable and does not
prove. The `Same` branch still estimates power from a single realization of this
run's own noise rather than averaging over the noise's distribution, so it is a
statement about *this run* — which is what F53 says any such statement can be.
And the pooled path, for comparisons assembled from separate sessions, still
resamples: it has no pairs to take signs of.

## 58 · F58 — A null result reaches the disk as a result (A9, B-086, B-213, §6.3, D16, F56)

**The failure this is against.** Not that MCF prints the wrong word. It is that
a null result is never written down — so that six weeks later nobody can tell
whether two configurations were compared and found the same, or were never
compared at all. **In an empty register those two look identical**, and the
second is the one that gets the work repeated.

**Two kinds, and neither is a failure.** `EntryKind::Comparison` and
`EntryKind::FitmentPlanned`. A9 names both halves — *"no measurable difference"
and "does not fit here" are findings, not failures* — and §6.3 already calls
*this will not run here, because it needs 131 GiB and you have 24* a complete
success of §III. A test asserts of both kinds that they are not
`EntryKind::Failure` and that their rendered line contains no such word,
because filing a null result under failures is filing it where nobody looking
for results will find it.

**Four outcomes for a comparison, all written.** *Differ*, *same to a stated
resolution*, *not yet decided*, and *not comparable*. The third and fourth are
the ones a reader will call *it didn't work*: the log says which it was rather
than leaving anyone to infer it from a missing number. The null result carries
the resolution that would have shown — *no difference as large as 5.0%, which
is a result, not a failure to find one* — so it never reads as *we stopped
looking*. The refusal names the conditions that differed and contains no delta
anywhere in the line, which is A8 held at the surface as well as in the type.

**The distribution goes to the disk, not only the verdict.** Every pair: both
raw durations, both positions, which arm ran first, and the difference. D16
keeps raw trials always and B56 derives summaries at query time, and this is
the case that proves why — **the stopping condition's own rule has changed
twice in two days** (F55, then F57). A record holding only the verdicts would
now be a record of two obsolete opinions. It holds the numbers, so the question
can be asked again.

Both arms' full conditions are written too, so the isolation question (F56) can
be re-asked rather than trusted.

**A plan is kept whichever way it came out.** Every variant of a repository,
with its outcome and the arithmetic behind it: what it needs, what is left, the
longest context that would fit, or how much more memory the machine would have
to have. Kept for the ones that fit as well as the ones that do not, because a
record of only the refusals cannot answer *when was this last known to fit*
(A1). `mcf pull` on a repository with no file named now writes one before it
answers, and a record it cannot open does not stop it answering — a plan is
information about a repository and a machine, not a change to either (A4).

**What was not established.** The plan is recorded on the listing path only;
`mcf pull` of a named file re-plans to check what is true now and does not write
that second plan, which would be two entries about one moment. Nothing yet reads
these entries back to answer a question — *what has this machine been told it
cannot run* is a query the register can now support and does not yet offer, and
that is B-217's. And a comparison entry is written by whoever holds a
`Comparison`; no surface produces one yet, because the benchmark runner is
B-080.

## 59 · F59 — The benchmark runner, and the first real comparison it refused to over-report (B-080, A18, §6.7, B65, B-091, F53, F57)

**What was built.** `mcf bench <model> --against <model>` — the runner A18 says
must exist and must never gate. It builds the comparison the only way one can
be built (B-250), stops when this run's own arithmetic decides (F55, F57), and
writes what it found to the record whichever way that was (B-086).

**It has no verdict that fails.** *They differ*, *they are the same to a stated
resolution* and *not decided* all exit zero, because all three are things the
machine said and none of them is MCF being wrong. What does fail is MCF being
unable to run the benchmark: no such model, no daemon, an engine that cannot
report a speed, an engine that refused. `checks/tests/benchmarks_never_gate.rs`
holds both halves of A18 — no verdict reaches a failing exit status, no gating
test bounds a wall-clock reading, no gating test writes to the machine's own
record, and the gating tier does not run the benchmark at all. The measured
tiers that *may* assert on timings are named in that file, because an exemption
written down is one somebody can argue with.

**The engine is asked, not assumed.** B65 and D31: MCF's own stand-in is
written to be read rather than to be fast. So before a trial is timed, one
request goes to each arm and the *account* is read for which engine actually
ran. A stand-in is refused by name rather than marked, because a marked number
is a number somebody will quote without its mark. On this machine, before
anything was provisioned:

```
mcf: …/stories260K.gguf would run on MCF's own stand-in, build 0.1.0-m0, and a
stand-in's answer can never be a speed (B65, D31)
```

**Then a real engine, and a real comparison.** `mcf provision llama.cpp` built
the pinned reference in a container; `mcf pull` acquired `stories15M-q4_0.gguf`
and `stories15M-q8_0.gguf` from `ggml-org/tiny-llamas` — two quantizations of
one model, which is exactly the single-variable comparison §3.4 wants and
B-091's shape in miniature. At 256 tokens, three consecutive runs:

```
no difference as large as 5.0%, after 20 paired trial(s) — the measured difference was 3.2%
no difference as large as 5.0%, after 25 paired trial(s) — the measured difference was 3.3%
no difference as large as 5.0%, after 12 paired trial(s) — the measured difference was 1.6%
```

Twenty, twenty-five, twelve — F53 again, the count is a property of the
sitting. **This is a null result, and it is the point**: a real benchmark on a
real engine, reported as a finding rather than as a failure to find one, and
recorded (A9, B-086).

**The defect the real run found.** Before the size test existed, the same
comparison at 64 tokens came back *they differ by 0.8%, after 113 paired
trials* — a real difference, found honestly by the sign test, and **an answer
to a question nobody asked**. A caller who says *resolving five percent* has
said that eight tenths of one is beneath notice; reporting it invites acting on
it, which is the §3.28 failure from the other direction. `Differ` now requires
the effect to be **real and at least as large as the resolution asked about**,
and below that the verdict is the null one — *no difference as large as five
percent* is true of a measured eight tenths. The measurement travels inside the
null result rather than being discarded (A1), so a reader who later cares about
a smaller resolution has it.

That the sign test could find 0.8% at all is F57's stated cost arriving as
designed: it uses only which arm won each pair, so a small consistent
difference becomes significant with enough pairs. The fix is not a weaker test;
it is asking the caller's question rather than the test's.

**The first unread condition read.** F56 left `mcf bench` reporting *isolation
is undetermined* over ten conditions. It now reads each file's **own tensor
types** — the file's quantization rather than the name a repository gave it
(A21) — so the sentence has become *quantization differs, and nine conditions
could not be compared*, and the comparison names what it isolated. Nine to go,
and each is a producer that does not exist yet (B-007, B-013).

**A later run, and what five of them together say.** The same comparison was
run twice more at a hundred and twenty-eight tokens, once while this shared
machine sat at a load average of sixty-eight and twice at thirty-five:

```
load 68   they differ by 29.2%, after 74 paired trial(s) — noise alone produced
          a gap that big 4.7% of the time      · 46 pairs one way, 28 the other
load 35   no difference as large as 5.0%, after 12 paired trial(s) — measured 2.7%
load 35   no difference as large as  5.0%, after  9 paired trial(s) — measured 2.9%
```

Five runs now: four say *no difference as large as five percent*, with a
consistent lean of 1.6% to 3.3% in the same direction, and one says 29.2% at a
false-alarm rate of 4.7% — three tenths of a percent inside the threshold it
had to clear. **One run at one-in-twenty is what one-in-twenty means**, and
nothing here distinguishes that from §3.27's own caveat, which predicts exactly
this: *a ratio measured at one level of contention need not hold at another,
because degradation is not uniform.* The loaded run's raw pairs include one at
a factor of thirty-seven, which is the machine being savaged rather than either
model being slow.

So the honest statement is the weaker one: **on this machine, at these token
budgets, q4\_0 and q8\_0 of stories15M differ by less than five percent, with a
small consistent lean; and one run under heavy contention crossed the threshold
in a way one run cannot tell from luck.** Which of the two it was is L25's
question and B-091's, and neither is answered here.

**What was not established.** A 15-million-parameter model at 64 to 256 tokens
is dominated by process start and request overhead, so *q4\_0 and q8\_0 are the
same here to five percent* is a finding about this configuration and not about
quantization — B-091 is the frontier and this is not it. Only one prompt, one
machine, one engine build, and both arms served by the same daemon. The
benchmark compares two *models* under identical settings; comparing one model
under two settings is a different argument shape the command does not have. And
the ceiling of two hundred paired trials and the default five-percent
resolution are chosen numbers, each stated in one line.

## 60 · F60 — The clock in the type stopped a comparison, not a record (A11, B-082, D9, F59)

**What was found.** `ClockKind` has put the clock in the type since the first
week: `Duration<Monotonic>` and `Duration<Simulated>` are different types, so a
simulated interval cannot be compared with a real one, stored where one is
expected, or averaged into a set of them. That was taken to satisfy A11 —
*nothing reported as a performance number may originate in simulation*.

It did not. F59's benchmark encoder was written generic over the clock, and a
generic encoder will happily put the laboratory's arithmetic into the record,
where it becomes a measurement with nothing on it to say otherwise. **The type
system was stopping the wrong operation**: comparison, which nobody was
attempting, rather than *writing down*, which the new code did in three lines.

**What closed it.** `Measurable`, a trait implemented for `Monotonic` and for
nothing else, and the comparison encoder is bounded by it. Encoding a
laboratory comparison is now a compile error, which is B-082's *the type system
prevents a simulated timing being published* in the strongest available form.
It bit immediately: three of the encoder's own tests were written on the
simulated clock — the shape questions do not care which clock they use — and
stopped compiling. They state monotonic durations now, which is the documented
seam and is in one place.

A trait somebody has to write, rather than a boolean somebody can set: adding a
clock to the publishable set is a decision about what MCF is willing to call a
measurement, and a check asserts that `Monotonic` is still the only member.

**And the same question asked of the whole tree.** No crate that writes to the
record names `Simulated` at all — not `mcf-record`, `mcf-serve`, `mcf-cli`,
`mcf-hub`, `mcf-standin` or `mcf-helper` — and the laboratory does not depend on
the crate that encodes comparisons, so the edge that would carry one does not
exist. Both are checked, because both are true today and neither is true by
construction.

**What was not established.** The bound is on the comparison encoder because
that is the only path that writes timings *as measurements* today; `self_cost`
and the record's clock-anomaly detector hold `Monotonic` concretely rather than
generically, which is the same guarantee reached a different way and is not
enforced by a bound. `Duration::from_nanos` remains available for any clock —
it is how a test states a known interval and how the laboratory states a
simulated one — so a test can fabricate a monotonic duration on purpose. That
is deliberate and is the one seam; what it cannot do is fabricate one by
accident from the laboratory's clock.

## 61 · F61 — The seed set had to become arithmetic, and EINTR was being called a cut-off transfer (B-290, B61, D19, A2, F53, F55)

**What B61 forbids.** Thirty trials at one fixed seed with identical inputs
produce thirty identical outputs: `n=1` wearing the costume of `n=30`, with a
spread of zero that reads as remarkable consistency and is an artefact. Fixing a
seed does not reduce variance; it conceals it, at the exact point §3.4 requires
uncertainty to be reported.

**A trial now cannot exist without saying what it drew.** `Trial` has a fifth
field with no default, so the artefact is not something a run can produce
quietly. The field is a `Draw` and it has two variants, because D19 gives the
two kinds of laboratory opposite rules: a behaviour trial draws seed *i* at
trial *i* from a declared set, and a timing trial **holds its seed still and
pins its generation length instead** — *a timing that varies because one run
stopped earlier is measuring the stop, not the speed.* Two variants rather than
one with a flag, so that a reader can never mistake a timing trial's fixed seed
for a behaviour trial's mistake.

**D19 assumed a trial count, and F55 had already taken it away.** *"The set's
size is the trial count"* was written when a benchmark was expected to declare
its repeats. It cannot: F53 measured the same command needing seven repeats in
one sitting and over a hundred in another, and F55 replaced the count with a
stopping condition that finds out as it goes. A fixed list of thirty seeds runs
out on the thirty-first trial, and *what happens then* has only bad answers —
wrap around and repeat a trajectory, or stop measuring because the list ended.

So MCF's published set is **stated as arithmetic rather than as a list**: a
stride and two mix rounds, six lines, unbounded, identical on every machine.
Everything D19 asked of a list it gives, and one thing more — **every step is a
bijection on `u64`**, so two trials cannot draw the same seed. A list of
literals could only promise that by being checked; here it is the arithmetic.
A test walks the first hundred thousand seeds and finds no collision, which is
not a proof and would catch a mistranscribed constant; the first four seeds are
pinned in a test because changing them breaks comparability with every
measurement already recorded against `mcf-standard-v1`.

A declared set is still admitted — a laboratory may need to reproduce somebody
else's run, and refusing would make MCF unable to check another tool's work.
It is refused if it repeats a seed, holds fewer than two, or has no name, and it
**runs out rather than wrapping around**: the runner stops, because repeating
the list would repeat a trajectory.

**The seed set is a condition, and comparisons check it.** D19: *comparisons
require matching seed sets the way they require matching hardware — recorded,
checked, and refused when they differ.* It is the floor's twelfth question, so
it renders on every surface and enters the isolation check for free (F56); and
`Comparison::from_trials` refuses two arms that drew from different sets by
name, with a refusal that says why a longer run does not fix it. The check runs
pair by pair rather than once at the end, because the two runs of a pair must
have drawn the *same* thing — what differs between them has to be the arm and
not the trajectory.

**A timing run answers the seed-set question, and the answer is *none*.** A7
governs values MCF could not read; a run that held its seed still knows
perfectly well what it did. Recording that as `Unknown` would have put a
deliberate discipline in the same box as a failure to look, and would have made
every timing comparison's isolation undetermined for ever — a wrong answer
rather than a cautious one.

**And `mcf bench` now pins a length.** It did not: `--limit` was optional and
defaulted to *as the daemon chooses*, which is a benchmark whose arms stop where
they like and whose timing therefore includes the models' verbosity. It pins
128 tokens when nobody says, states the discipline in its own report, and
records it.

---

**The second half of this finding is a defect the suite found under load, and
it is not the one it looked like.** A full workspace run on a machine at load
sixty produced one divergence: the stall scenario, which exists to show that
*MCF's deadline ends the wait rather than the far end*, came back
`transfer.interrupted` instead of `transfer.stalled`.

**It did not reproduce**: three thousand four hundred targeted runs, the last
three thousand under ninety-six deliberate burners, all produced what the
scenario declares. So what caused it is **not established**, and saying
otherwise would be inventing a mechanism to fit one observation.

What the investigation did find, by reading the contract of `read` rather than
by measurement, is a real misclassification on the same path.
`ErrorKind::Interrupted` is EINTR: a signal arrived while the thread was blocked
in the kernel and the read did not happen. It is the one io error whose contract
is *retry*, and MCF was classifying it as `transfer.interrupted` — telling an
operator on a busy machine that their download had been cut off by something
that was not there. A2 forbids the wrong answer stated confidently as firmly as
it forbids silence. Reads on the wire now retry it and classify everything else
exactly as before, with two tests: one reader that is interrupted twice and then
answers, and one that resets and must still be reported.

Whether that was the divergence is unknown. It is a defect either way, and it
removes one candidate.

**What was not established.** The published set's bijectivity is argued from the
structure of its three steps and checked over a prefix, not proved. The
divergence above has one observation and no mechanism. And nothing yet validates
the standard set against a larger random one, which D19 requires periodically
and which is B-291's.

## 62 · F62 — The seed set is shown representative, and the sampler that would have cleared it for nothing (B-291, D19, §6.16, §7.13, B65)

**What was asked.** D19 gives MCF a published seed set and then puts §6.16 on
it: *the instrument does not get to grade itself.* The set is the first
thirty-two draws of a stated stream (F61); the question is whether that prefix
behaves like the stream at large, or whether MCF has been drawing from an
unlucky corner of it since the day it was written.

**What was run.** Thirty-two seeds from the head of the stream against three
hundred and twenty from a million indices further along — two draws from one
space, which is exactly what *is the prefix representative* means, and
reproducible in a way a genuinely random draw would not be (§3.12). The
per-trial outcome is how many distinct tokens the generation used: a behaviour
statistic, never a speed, which is what a stand-in may legitimately produce
(B65, D31). One prompt, held still, so that what varies between trials is the
seed and nothing else (A8).

```
sampler: Nucleus { temperature: 1.0, top_p: 0.95 }, 48 tokens, stories15M-q4_0
the standard set is indistinguishable from a draw 10 times larger,
to within 10.0% — 32 trials against 320
```

Three hundred and fifty-two generations, eleven and a half minutes in release
on this machine. **The set stays**, and it stays because it was shown rather
than because nobody looked.

**The defect this nearly had.** The first version of the tier was going to use
MCF's shipped generation, which is `Settings::Greedy` — and **greedy ignores
the seed entirely**. Every seed produces the same tokens, so both draws would
have been three hundred and fifty-two copies of one number, the pooled null
would have found no difference between two constants, and the tier would have
cleared the seed set *for a reason that has nothing to do with the seed set*.
A green check that cannot fail is worse than no check, because it is read as
evidence.

The tier therefore names a stochastic sampler explicitly, and says in its own
header that this is the line to change when MCF ships a stochastic default.
Which brings out something worth stating plainly: **today the seed set changes
nothing about any MCF generation**, because the sampler MCF ships is greedy. It
is recorded as a condition, it is checked before two comparisons are put side
by side, and it will start mattering the day a sweep picks a sampler that draws
(D18, B-281).

**The polarity is inverted, and the type says so.** Everywhere else in
`mcf-bench`, *they differ* is the interesting answer and *the same* is a null
result. Here the clearance is *the same to within a stated resolution* and the
finding is *distinguishable*. `Representative` is its own type rather than a
reused `Verdict` for that reason: a green result must not be able to render as
a discovery, and a test asserts that neither rendering contains the other's
word.

**And *not decided* is not clearance.** A run that could not separate the two
has not shown anything, and treating it as clearance is precisely how an
unvalidated instrument stays unvalidated. `clears_the_set` is true for one
variant only, and the tier fails on the other two.

**The two draws cannot be paired**, and the module says so: trial *i* of each
shares nothing but its index. So it uses the pooled null — the construction
§3.27 calls weaker everywhere else — which is the honest one here, because
there is genuinely nothing to pair. A test also asserts the two draws share no
seed, which the stream's bijectivity gives and which would otherwise be
comparing part of the set with itself.

**What was not established.** One model, one prompt, one sampler setting, one
statistic. *Representative* here means *the distinct-token count of a 48-token
generation from this model does not differ by more than ten percent between the
prefix and the body* — a different statistic or a different model could give a
different answer, and D19 asks for this periodically rather than once for
exactly that reason. The larger draw is ten times the set, which is a chosen
ratio; the resolution of ten percent is chosen too, and both are one line each.
Eleven and a half minutes is the cost on a fifteen-million-parameter model, and
it scales with the model.

## 63 · F63 — The recommendation is in a different repository from the weights (B-281, B60, D18, A21, §3.15)

**What B60 asks for.** *Calibration adopts what the artifact recommends rather
than imposing a house style, and marks it declared, unverified until a sweep
has tested it.* Its violation is *a global default temperature applied to every
model, which measures each of them under settings some were never designed
for.*

**The first question is where a recommendation would be**, and the answer was
measured rather than assumed. There are exactly two places MCF can look: the
model file's own metadata, which travels with the weights and is the only
source an offline machine has; and the repository's `generation_config.json`,
which does not travel.

Six repositories were listed and one model file's metadata was dumped:

| repository | `config.json` | `generation_config.json` |
|---|---|---|
| `ggml-org/tiny-llamas` | no | no |
| `bartowski/SmolLM2-135M-Instruct-GGUF` | no | no |
| `unsloth/Qwen3-0.6B-GGUF` | yes | **no** |
| `unsloth/gemma-3-270m-it-GGUF` | no | no |
| `Felladrin/gguf-Llama-160M-Chat-v1` | no | no |
| `QuantFactory/SmolLM2-360M-Instruct-GGUF` | no | no |
| `HuggingFaceTB/SmolLM2-135M-Instruct` *(the base repository)* | yes | **yes, 132 bytes** |

And the GGUF metadata of an acquired model carries twenty keys — architecture,
tokenizer, quantization version — and **not one sampler parameter**.

**So: none of six GGUF repositories publishes a sampling recommendation, and
the one repository that does is the base repository the conversion came from —
which MCF was never asked to fetch and cannot identify from the conversion
alone.** B60's *adopt what the artifact recommends* has, for the artifacts MCF
actually acquires, nothing to adopt. That is a fact about the ecosystem rather
than about MCF, and it is the fact that shapes what MCF can honestly do.

**What was built.** MCF looks in both places and says what it found.
`mcf_standin::recommended` reads the file's metadata under the architecture it
declares; `mcf_hub::recommendation` reads the repository's
`generation_config.json`. Each has *nothing declared* as a state rather than as
an empty set, because *no recommendation* and *a recommendation that sets
nothing* are different facts and **only the first justifies MCF choosing for
itself**. The hub's reader keeps a third state as well: a file that is
published and states no sampler parameter is a publisher who looked and said
nothing, which is not the same as a publisher who did not look.

`mcf_core::configuration::Calibrated` is where the attribution lives: values
plus **whose choice they are**, with four constructors and no fifth — declared
by the artifact, measured here by a sweep, pinned by a laboratory, or MCF's own
with the reason it had to choose. There is no constructor that omits the
source, and `Sampling` has no `Default`, which together are B60's *no global
default* as a shape rather than a habit.

**And the surface says it.** `mcf explain` previously read *sampler: greedy —
MCF default, stated in crates/mcf-cli/src/run.rs*: transparent about the value
and silent about the fact that nobody had asked the model. It now reads

```
sampler   greedy   MCF's own, because this file recommends none: no sampler
                   key in its metadata, and a conversion repository publishes
                   no generation_config.json either (B60, F63). Stated in
                   crates/mcf-cli/src/run.rs
```

which is the same value and a different claim. §3.15's *no hidden choices* is
not satisfied by naming the value; it is satisfied by naming whose the value
is.

**Kept apart** (B60): a laboratory's pinned method is quarantined from anything
that inherited the artifact's recommendation, in both directions, and from
another laboratory's pin — two laboratories that each imposed their own are two
methods, not one. Everything that did not pin stays comparable, because an
artifact's recommendation, a sweep's measurement and MCF's own choice are all
answers to *how should this be sampled*, and A8 already refuses the case where
the answers differ.

**Attribution is not identity** (D17, D18). The same numbers are the same
distribution however each arrived at them, which is exactly why the attribution
has to travel separately rather than be inferred from the values — a test
asserts both halves.

**What was not established.** The half of B60 that needs a sweep: *divergence
between the recommendation and what measures best here is a finding and is
surfaced.* Nothing measures a sampling setting yet (B-280 is open), so
`Chosen::MeasuredHere` exists and is unreachable — which is the honest way to
say a state is defined and unpopulated. The GGUF keys are read where present
and no examined file has them, so that path is tested against constructed files
rather than against a real one. And MCF does not follow a conversion back to
its base repository to find the recommendation there: doing so would mean MCF
deciding which repository a file came from, which is a claim about provenance
it has no basis for (A21, §3.6).

## 64 · F64 — Every benchmark trial was cold, and three fifths of it was process start (B-081, §6.13, B-376, F59, D41, F35)

**§6.13's rule.** *Caching, reuse and adaptation are permitted — but anything
that could change a result must be visible in that result's conditions. A
measurement taken with a warm cache is a different measurement from one taken
cold, and MCF must know which it produced.* The corollary is sharper: **the
benchmark path may not adapt.**

MCF had the information and was not using it. The daemon's account has said
`loaded: resident_in_server` or `loaded_for_this_request` or
`per_request_subprocess` for some time; nothing read it, so nothing recorded it,
and every timing MCF has ever reported was silent about what it reused.

**What was built.** `mcf_bench::warmth`: each trial's warmth, read from the
daemon's own words rather than parsed loosely — a state this build has not been
taught is *unstated* rather than whichever of the two it superficially
resembles, and the wrong guess there would silently make a mixed run look
uniform. A run is `Uniform` or **`Mixed`**, and a mixed run says *this is not
one measurement (§6.13)* rather than averaging over it. **One warm trial among
a hundred and ninety-nine cold ones is a mixed run**: §6.13's concern is not
proportion, it is that the result depends on hidden history.

It becomes the condition floor's thirteenth question, written by
`Interleaving::finish` — the runner is what saw each trial's warmth, so the
runner is what records it and a caller cannot forget. From there it flows into
`Isolation` for nothing: **a warm arm against a cold one is confounded, and A8
withholds the delta.** §6.13's *must be visible in the conditions*, enforced
rather than printed.

**What it made visible, immediately.** Two runs on the provisioned engine:

```
q4_0 against q8_0     reuse   cold: every trial loaded the model for itself
q4_0 against itself   reuse   cold: every trial loaded the model for itself
```

Even comparing a model with *itself* — where residency should obviously hold —
every trial was cold. The account says why: `loaded: per_request_subprocess`.
The provisioned engine has a server path, and `mcf bench` **cannot reach it**:
a turn of identifiers goes to the server, a prompt goes to the completion tool,
and a prompt is what `mcf bench` sends (B-376). So the benchmark path is
uniformly cold **by construction**, which is at least reproducible and is not
what anybody would have assumed.

**How much of a trial that is.** The same model at two token budgets,
interleaved:

```
--limit   8    median 0.048 s
--limit 256    median 0.109 s
```

Two points on `t(n) = overhead + n × cost`: **about 46 ms of overhead and
0.25 ms per token**. At the benchmark's default of 128 tokens that is **60% of
every trial spent on process start and model load**; at the 64 tokens F59's
comparison used, **74%**.

**Which explains F59.** Two quantizations differ in the dequantization work
done *during generation* — the 40% — and are identical in the 60%. A real
difference of, say, ten percent in generation shows up as four percent of the
measured total, which is under the five percent that comparison was asking
about. F59's null result is not evidence that the quantizations run alike; it
is a measurement of a quantity three fifths of which cannot differ between the
arms. **The condition was there all along and nothing was reading it.**

**What was not established.** The 46 ms includes `mcf run`'s own process start,
because the instrument spawns it — the daemon-side share is a subset and is not
separated here. Two points fit a straight line and two points cannot show it is
one; a third budget would. Warm behaviour is untested against a real engine
because the benchmark cannot currently produce it, so `Warmth::Warm` is
exercised against constructed runs. And making the benchmark warm is not this
item's: it means tokenizing MCF-side and sending identifiers so the request
reaches the server path, which changes what is measured and belongs with
B-376 and B-090.

## 65 · F65 — One resident model and paired interleaving cannot both be had (B-090, B-081, B-250, DEC-001, §6.13, B53, F64)

**What was tried.** F64 established that every benchmark trial was cold and
that three fifths of a default trial was process start, because a *prompt*
routes to the provisioned engine's completion tool — a fresh process each time
— while a *turn of identifiers* routes to its server, which stays up (B-376).
The obvious fix: tokenize the prompt MCF-side, once per arm, and send
identifiers.

**It made the benchmark worse, and the condition said so immediately.**

```
reuse   MIXED: 294 trial(s) loaded the model, 106 found it resident,
        0 unstated — this is not one measurement (§6.13)
```

The server holds **one** model at a time (DEC-001: *two make every latency
figure depend on what else was loaded*), and a paired comparison alternates
two. So a trial is warm exactly when the previous request used the same model —
which happens only where the drawn order puts two runs of one arm next to each
other across a pair boundary. About a third of trials, and **which third is a
property of the order the run drew**.

That is a collision between two rules that are each right:

- **B53** randomizes which arm goes first in each pair, so that going first is
  not an advantage.
- **§6.13** forbids a measurement whose result depends on hidden history.

Randomizing the order *is* the hidden history here. Strict alternation would
make every trial cold and uniform — and would give going first a fixed
position, which B53 forbids for a different and equally good reason.

**So: with one resident model, a paired interleaved comparison of two models
cannot be warm.** It can be uniformly cold, or it can be mixed. There is no
third option on MCF's current serving design, and that is an architectural
consequence rather than a defect in any of the three rules.

**What was built, given that.** The delta is withheld from a mixed run, exactly
as it is from a confound: `Withheld::MixedReuse`, and the rendering leads with
*no delta* rather than a number. A mixed run is **not declarable** — an
operator can say *I know these two variables moved together*, which answers
A8's question, and cannot say *I know some of my trials loaded the model*,
because that is a statement about the instrument rather than about the
question. And the run stops as soon as it goes mixed rather than spending its
ceiling to arrive at the same refusal.

Two ways out, and the refusal names both:

```
--cold, two models      reuse  cold: every trial loaded the model for itself
                        no difference as large as 5.0%, after 9 paired trial(s)

one model against       reuse  warm: the model was already resident for every trial
itself, warm path       no difference as large as 5.0%, after 15 paired trial(s)
```

`--cold` sends text, which is a fresh process per request and therefore
uniform: a measurement that includes F64's forty-six milliseconds as a stated
condition, which is a measurement. Comparing a model with *itself* — the noise
floor, and the control every comparison should be able to run — never changes
the resident model and is uniformly warm.

**Uniform and honest beats warm and mixed**, and the point of B-081's condition
is that MCF can now tell which it has.

**What was not established.** Whether a warm two-model comparison is worth
having is DEC-001's question and not answered here: it would need two resident
models, which DEC-001 refused for serving and did not consider for measuring.
The nine and fifteen paired trials above are single runs and F53 applies —
those counts are properties of the sitting. And `--cold` measures generation
*plus* process start; separating them would need the server path, which is the
thing that cannot be uniform.

## 66 · F66 — Fifty-eight of a hundred trials is fifty-eight data points, and the fifty-ninth was not one (A4, B-087, §3.1, A1, A6)

**A4 is absolute and names its own violation:** *an all-or-nothing return type
on anything that can partially succeed.* MCF had written exactly that three
days earlier. `mcf bench`'s runner returned `Result<Comparison, String>` and
discarded every completed pair the moment one request failed — a hundred paired
trials thrown away because the hundred-and-first did not answer.

Nothing about it looked wrong. A loop that gives up on an error and returns it
is the natural thing to write, which is why A4 is absolute and why the check is
about the *shape* rather than about any particular loop.

**What was built.** The runner always yields the comparison it built, however
far it got, and the reason it stopped travels **with** the trials rather than
instead of them: `Comparison::cut_short`, an `Option<String>` so that *it
finished* and *it was interrupted and nobody recorded why* stay different facts
(A7). It reaches the record as `cut_short`, where `null` means the run
finished — so six weeks later a short run that was interrupted is
distinguishable from a short run that decided quickly.

**Demonstrated by interrupting one.** The daemon was stopped five seconds into
a benchmark:

```
not decided after 58 paired trial(s): the arms have not separated …
  reuse    warm: the model was already resident for every trial
  pairs    58 interleaved, order drawn per pair
  CUT SHORT after 58 pair(s)
           the stream from the daemon ended before its account …
```

Fifty-eight pairs kept, the verdict over them standing, and what was lost said
rather than implied by a smaller number. Each pair is two runs of two arms
taken back to back under the same conditions, and an interruption afterwards
does not reach back and unmake them.

**The second defect, which the first run of the fix exposed.** The interrupted
request was being recorded as a trial with a duration of **zero nanoseconds**.
It showed up as the run going `MIXED` — the failed request's warmth was
*unstated*, so ninety-three warm trials plus one unknown was not one
measurement — and behind that flag was something worse: a number nobody
measured had entered the distribution.

A4 says a partial outcome is preserved. It does not say a non-outcome is one.
**A run that did not happen is not a trial**, and a zero-duration stand-in for
it is A1's forbidden loss wearing a data point's clothes. The runner now
records no pair at all where a run did not happen, and does not attempt the
second run of a pair whose first did not happen — one run alone is not half a
pair (§3.27). After the fix the same interruption leaves fifty-eight *warm*
pairs and no fabricated zero.

**And a third, found while writing the check for the first two.** The record
was written from a finding **recomputed at the default resolution** rather than
the one the operator asked about. A caller asking about half a percent was
shown one verdict and the record kept another: two answers to one question
(A6), in the worst possible place to have them. The record now carries the
finding that was displayed.

**What was not established.** The interruption tested here is the daemon
stopping; a trial that fails for another reason takes the same path but has not
been run. `checks/tests/partial_outcomes_are_outcomes.rs` checks the shape
rather than the behaviour — the behaviour is in `mcf-bench`'s own tests, and
the shape is what stops the natural all-or-nothing return coming back. And A4's
other example, *eleven tokens before a runtime died are eleven tokens*, was
already held by the generation path and is asserted rather than newly built.

## 67 · F67 — The first frontier, and what it is mostly a frontier of (B-091, §XII, §3.4, §3.27, F64, F65)

**The cleanest comparison §3.4 admits**, and MCF's first: one model, one
machine, one prompt, one engine build, one sitting, and seven quantizations of
the same weights by the same publisher. What differs between two arms is the
quantization, read from each file's own tensor types rather than from its name
(A21), and everything else is held still.

Built as **seven paired comparisons against one reference arm**, not as seven
absolutes on a chart: §3.27 makes the comparison the durable output, so each
point carries its own stopping condition, its own count and its own conditions.
`--cold` throughout, because F65 established that a warm run of two models
comes out mixed and has no delta to give.

```
reference  SmolLM2-135M-Instruct-Q8_0.gguf   (145 MB)

f16      271 MB   the reference is quicker by 61.9%   6 pairs, 3.1%
Q2_K      88 MB   this arm is quicker by 26.1%        6 pairs, 3.1%
Q3_K_S    88 MB   this arm is quicker by 25.7%        6 pairs, 3.1%
Q4_0      92 MB   this arm is quicker by 24.4%        6 pairs, 3.1%
Q4_K_M   105 MB   this arm is quicker by 18.2%        6 pairs, 3.1%
Q5_K_M   112 MB   this arm is quicker by 16.6%        6 pairs, 3.1%
Q6_K     138 MB   no difference as large as 5.0%; measured 2.8%
```

Absolute, for the one point that has it here: **372.0 ms against 476.7 ms**,
which §3.27 keeps local and does not travel.

**It is monotone in file size, and that is the finding.** 88, 88, 92, 105, 112,
138, 145, 271 megabytes give 26.1, 25.7, 24.4, 18.2, 16.6, ~3, 0, −61.9
percent. The ordering is exact and the two 88 MB files — which are different
quantization *schemes* — differ by four tenths of a percent from each other.

F64 already said why: every trial loads the model, because the engine holds one
at a time and a paired comparison alternates two, and load time goes with
bytes. So **this is principally a frontier of file size**, and the generation
work that distinguishes one quantization scheme from another is the smaller
term underneath it. A straight line through the two extreme points predicts f16
at 1.84× the reference against 2.04× observed, so it is not *purely* size —
f16 also moves more memory per token — but size is what dominates.

That is a true measurement, honestly conditioned, and it is not the frontier
somebody wants. The frontier somebody wants needs the load out of the figure,
which needs a warm two-model comparison, which needs two resident models
(F65, DEC-001).

**Six paired trials, seven times.** The sign test's minimum: six pairs won by
one arm is one chance in thirty-two, which clears one in twenty, and every
difference here is far larger than the five percent asked about. The
comparisons stopped at the first count that could decide, which is F55's
stopping condition doing exactly what it is for.

**A size without a direction is not a comparison.** The first run of this
frontier reported seven differences and said of none of them which way round it
was — a table nobody can read. `Verdict::Differ` carries which arm was quicker
now, the sign was already in the paired differences, and the rendering names the
arm. Found by producing the thing and looking at it.

**What this characterizes.** The instrument. `scripts/frontier.sh` prints that
in as many words, because B-091 requires it: this is not a statement about
quantization in general, about these quantizations on other hardware, or about
anything but latency — and what a quantization costs in *quality* is not
measured here and is not measured anywhere yet (§IV, DEC-002).

**What was not established.** One sitting, and F53 says a sitting is what a
sitting had. One prompt of four tokens and one budget of 128, so the ratio of
prompt to generation is fixed and unexplored. The reference arm is Q8_0 by
default and a different reference would give a different table of the same
shape. Seven of the twenty variants this repository publishes. And the script
is not a step of `scripts/ci.sh` and never will be: it has no pass condition,
and A18 forbids a benchmark gating a change.

## 68 · F68 — The bundle's header said it held no user content while carrying the prompt (B-211, PR2, A24, A25, §II)

**§II's fourth obligation, built.** *Every measurement carries a stated method,
stated conditions, stated uncertainty, and the ability for someone else to
repeat it.* Three were built; the fourth was a property of the design rather
than a thing anybody could hand over. `mcf bundle <entry-id>` is that thing: one
file holding a claim and everything it rests on, selected out of the record
rather than assembled beside it.

```
wrote /tmp/comparison_2026-08-28T12-48-45Z_f67204b1_0000.mcf-bundle
  5 entr(ies), sha256 3075435016fa04…

  the claim   … the left arm is quicker by 26.2%, after 6 paired trial(s)
  rests on    component_provisioned_2026-08-28T06-39-11Z_ef027fef_0000
  rests on    artifact_acquired_2026-08-28T12-22-48Z_e105a153_0000
  rests on    artifact_acquired_2026-08-28T12-23-32Z_4ce3df53_0000
  rests on    machine_profile_2026-08-28T12-47-38Z_a1171665_0000
```

**One mechanism, not a fourth.** B-302 requires export, contribution and repro
bundle be one thing, because three serializations of the same evidence
eventually disagree about what the evidence was. `export::write_selected` is
`write` with a predicate, and the module's own claim — *the kinds differ in
what is selected, never in how it is written* — stopped being a sentence in a
header and became a function signature.

**The method had to start being recorded.** A comparison's conditions say what
the machine was; nothing said what the *question* was. A floor full of hardware
does not tell somebody what to run, so `Method` — the prompt, the resolution
asked about, the ceiling, the engine asked for, whether every trial was made to
load the model — is written with every comparison now. It is deliberately not
part of `Conditions`: two people asking different questions of one machine are
not two conditions, they are two experiments.

**And that made the header lie.** The bundle's `contains_user_content` was a
constant `false`, on reasoning that was correct when it was written: *this
module reads the journal, and the journal is not the content store* (A25,
B-161). The journal still is not the content store. But **the method of a
measurement is text the operator wrote**, and it has to travel or the bundle
reproduces nothing — so the first bundle produced carried a prompt under a
header saying it carried no user content.

A `false` that is sometimes wrong is worse than no field at all: a reader
deciding whether to send a file is entitled to know what leaves with it (A24,
§3.20), and that field is the one they will read. It is computed from the
entries now, over a named list of the places operator text reaches the record —
named rather than guessed, because guessing gets it wrong in the direction that
matters, and a check requires the list stay complete.

**What leaves with it is printed before it leaves.** Producing a bundle is not
publication and is not gated — writing a file to a path the operator named is
not sending it. What the surface owes is the other half of A24: the list a
person should read first, which names the two things they would not expect —
the prompt they wrote, and this machine's **full hardware identity**, which a
*contribution* strips and a bundle deliberately keeps. That difference is what
makes a bundle checkable rather than aggregable, and it is the reason the two
artifacts are opposites.

**What is missing is said too.** The first bundle produced had no machine
profile, because nothing had written one on this machine — so it says *NOT in
it: what this machine is*, and names the command that would fix it. A bundle
whose conditions lack the machine is a weaker artifact than one whose do not,
and that is the reader's to weigh rather than to discover later (A7).

**What was not established.** The bundle carries the identifiers of what it
rests on and not the weights: PR2 anticipated that, and it makes verification
depend on fetching them, which §7.11 cares about. The join between a claim and
an acquisition is the artifact's path, because the record holds no foreign key —
a bundle for a claim about a model acquired on another machine, or moved by
hand, will carry no provenance and does not currently say so. And re-running the
method on another machine is `mcf verify`, which is B-212 and not this.

## 69 · F69 — A band predicted before the measurement, and the measurement landed in it (B-214, B-215, PR3, A20, B46, §6.16, F67)

**PR3's question, asked before a byte is fetched:** *how fast would this be
here?* MCF cannot measure a model it does not have, and refusing to say
anything makes choosing between twenty published quantizations cost tens of
gigabytes a guess. A20 admits the middle answer and then walls it off: *an
estimate can never be mistaken for a measurement, never promoted into one, and
never compared with one. It can only be replaced by one.*

**The prediction, made first.** A quantization this machine had never
benchmarked — `SmolLM2-135M-Instruct-Q4_K_S`, 102,039,904 bytes — was projected
from the record with itself excluded, by reading between the two measured sizes
it sits between:

```
 91,893,088 bytes   381.3 .. 395.9 ms   (measured)
105,454,432 bytes   387.2 .. 407.4 ms   (measured)

PREDICTION for 102,039,904 bytes:  between 385.7 and 404.5 ms
```

**Then measured.** `mcf bench` on the file, `--cold`, 128 tokens:

```
medians  397.3 ms and 471.4 ms
```

**397.3 ms, inside a band of 385.7 to 404.5** — about three fifths of the way
across it. One prediction, made before the measurement and not adjusted after.

**What makes it defensible is what it refuses to do.** It reads *between*
measured points and never past them. F67 established the relationship this
rests on — latency monotone in file size, because every trial loads the model —
and F67 also measured where it stops being straight: a line through the
extremes predicted the largest point eleven percent low. So a file outside the
measured range gets no band at all, and the refusal says why. It is
band-shaped, because a duration predicted from a rate is a range and one number
is *the smallest possible version of a confident wrong number* (B46). It is
from local history only, because there is no corpus and B34 would let one
advise rather than decide. And a budget this machine has no history at gets
nothing, because two requests of different lengths are two different things.

The surface says the word: *this is an ESTIMATE read between two measured
sizes, and A20 forbids it standing beside a measurement or being promoted into
one.* A20's *clearly-labelled* is not satisfied by a type name nobody sees.

---

**And then §6.16 turned on it** (B-215). *The instrument does not get to grade
itself*, and a projection nobody scores is a claim MCF makes for ever without
finding out whether it was any good. `mcf doctor` now says:

```
PROJECTION, SCORED AGAINST WHAT WAS LATER MEASURED
  42 of 49 measured points fall inside the band that would have been
  projected for them; the worst miss is 150.1% (5 could not be scored:
  nothing measured on one side of them)
```

**Scored by leaving each point out**, which needs no stored predictions and no
new record kind: for every measurement in the history, the band its neighbours
would have given is computed and compared with what it actually was. It is
recomputed from the record each time it is asked, so it moves as the history
does — which is what *over time* means when the history is the thing changing.
A stored score would be a score about a record that has since changed.

The points at the ends are **not scored and not counted as misses**: with them
left out there is nothing to read between, so there is no projection to score,
and counting them would be scoring the refusal to extrapolate — the thing the
model gets right.

**Eighty-six percent, and a worst miss of 150%.** That is a finding about the
model rather than a reassurance: the misses are real and the largest is large.
The history includes points taken while this shared machine was at a load
average of sixty, whose slowest trials ran to thirteen seconds against a
fastest of seventy-nine milliseconds — a band that wide is not a band, and a
neighbour interpolated from it is not a prediction. The score says so instead
of hiding it.

**What was not established.** One prediction checked against one measurement,
on one family, at one budget, in one sitting. The relationship is F67's and is
principally about file size because every trial loads the model — a projection
built on a warm engine would be projecting something else. The band's width is
the fastest and slowest trials seen, which is honest and crude: it inherits
every outlier the history has, and the 150% miss is one of them arriving. And
nothing prunes the history: a measurement taken under contention stays a point
for ever, which is a decision nobody has made (DEC-013).

## 70 · F70 — A run that cannot decide names what it competed with, and I could not make one (B-216, PR5, §3.8, B24, B4, D25, F55)

**B24's refusal, upgraded to a diagnosis.** §3.8: *MCF knows the difference
between "this model is slow" and "this machine was busy". When it cannot tell
the difference, it says so rather than attributing the result.* Having been told
a measurement is unattributable, the operator's next question is always **by
what?** — and MCF is the only thing positioned to answer, because it was there
and nothing else was.

**What was built.** `mcf_core::hardware::contention` reads `/proc` twice a
stated fifth of a second apart, turns accumulated processor time into a rate,
and **names the processes**: a snapshot whose answer is *the machine was busy*
is a number, and one that says which processes and what they took is a
diagnosis. The kernel's own stall accounting is read for processor, memory and
storage. Per-process accelerator occupancy is `Unknown`, because MCF has no
vendor library here and D25 makes *unknown* not *none*.

**MCF's own process is marked, not filtered out.** It is the one process the
reader can do something about, and a snapshot that hid it would be hiding the
useful half.

**Two rules had to be satisfied at once.** B4 refuses ambient sampling and D5
settled it: MCF does not watch, it looks when there is a reason. So a snapshot
is taken **because a comparison could not decide**, and at no other time — one
caller, no timer, no thread, and *after* the run, because sampling during one
would make MCF one of the competitors it reports. A check asserts each of those
and counts the callers, because the failure here is the natural one: a snapshot
is useful, so somebody takes it more often, and then on a timer, and then MCF
is a monitor.

**And it goes into the record**, as `contention_snapshot`. PR5 is explicit that
the snapshot must persist rather than be a transient thing on a screen, because
a finding printed and not written down does not survive the terminal (§3.1).

---

**What I could not do: make a real run end *not decided*.** Five attempts, all
of them trying to provoke the condition the snapshot exists for:

| resolution | budget | arms | load | outcome |
|---|---|---|---|---|
| 0.5% | 32 | two quantizations | 61 | decided at 113 pairs |
| 0.1% | 32 | two quantizations | 61 | *differ by 2.0%* at 12 pairs |
| 0.1% | 32 | one model against itself | 61 | *same* at 113 pairs |
| 0.05% | 1 | one model against itself | 61 | refused: the unit is one decimal place |
| 5% | 1 | two quantizations | 60 | *same* at 35 pairs |

**That is a finding about the stopping condition rather than about this path.**
F55 and F57 built it to find the count a claim needs, and on this machine, at a
load average of sixty, with the finest resolution the unit expresses, it reached
a verdict every time. *Not decided* is genuinely hard to reach — which is good
news about the instrument and inconvenient for demonstrating a feature that
fires only there.

So the rendering is covered deterministically instead: a comparison built to be
undecidable, a snapshot constructed by hand, and the section asserted — including
that a snapshot which could **not** be written says so rather than reading as
kept (A2). The sampling itself is tested against this real machine: that it
names the busiest first, that the total counts processes the list did not name,
that MCF's own process is marked, that an unreadable accelerator is unknown
rather than idle, and that it costs the interval it states and no more.

**A check that could not tell a type from a prefix.** `ContentionSnapshot`
begins with the six letters of `Content`, and
`checks/tests/content_is_not_the_record.rs` — which exists to stop the record
acquiring a way to hold user content (A25, §6.8) — refused it on a substring
match. The fix was to make the check precise rather than to rename around it: an
occurrence followed by a lowercase letter is part of a longer word.
**A check that blocks correct work teaches people to rename around it, and a
rule people rename around is a rule nobody believes.**

**What was not established.** The end-to-end trigger has not been observed
firing on a real run. Thermal and clock state are not in the snapshot: F53
measured that the only sensor this machine exposes reads sixteen degrees, which
is not a processor temperature, and a field filled from it would be worse than
an absent one. The interval and the number of processes named are chosen
numbers, each stated in one line. And the command lines go into the operator's
own record — §3.20's gate is on whatever *sends* a record, and `mcf bundle`
already lists what leaves.

## 71 · F71 — The machine either side of a run is a condition, not a gate (B-217, DEC-007, §3.4, §3.8, A6, A7)

**The item as written, and why it could not be built that way.** B-217 asked
for a *quiet-machine pre-flight*: a laboratory that refuses to begin on a
contended machine rather than producing an invalid result. Reshaping it in the
register already moved it once — not *refuse on a contended machine* but
*refuse on a machine that is not in its own steady state* — because a machine
with a baseline load of two cores is not a broken machine, it is somebody's
machine. Building even the reshaped version stops at the same wall: **refusing
requires a threshold, and a threshold is a figure MCF would be choosing.**
DEC-007 leaves that band open on purpose. A number invented here would deny a
result to every operator whose ordinary baseline sits above it, and would be
exactly the kind of figure that decision exists to derive from measurement
rather than from taste.

**So the measurement was built and the refusal was not.**
`mcf_core::hardware::contention::steadiness(readings)` takes *n* successive
contention samples and reports the middle reading, the spread between the
extremes as parts-per-million of that middle, and how many readings it took.
Its doc comment says the thing outright: *this measures; it does not judge.*
`Comparison` gained a `MachineHeld` — competing processor time before the run
and after it, with room for the steady-state readings on either side — and
`moved()` reports the shift as parts-per-million of the smaller figure. It is
recorded as a condition (§3.4, A6) and rendered as one:

```
machine  0.15 core(s) competing before, 0.15 after — the level moved 0.0%
         across the run, which is a condition and not a verdict: what movement
         is too much is DEC-007's open band
```

**Both halves were exercised on the real machine.** A run on the quiet machine
reported 0.15 cores before and 0.15 after, a movement of 0.0%. A run with forty
spinning processes started eight seconds *into* it reported 0.25 cores before
and 4.60 after — a movement of 1740% — and still produced its verdict, which
now arrives next to the fact that the floor moved by a factor of eighteen
underneath it. That is A7 rather than A5: MCF does not know that this verdict
is wrong, and saying so would be a claim it cannot support. It knows the
conditions were not held, and it says that instead.

**What this leaves open, deliberately.** The reader of such a record can
discard it; MCF cannot discard it for them. The gate B-217 originally wanted is
still the right eventual behaviour, and it stays blocked on DEC-007 — which now
has something to be decided *from*, since every comparison recorded from here
carries the movement its own machine showed. That is the intended direction of
travel: the measurement precedes the threshold, and B-217 stays in progress
until the threshold has a measured basis rather than an assumed one.

## 72 · F72 — Work is counted; minutes are derived, banded, and sometimes absent (B-224, B-225, B46, D14, A20, A7)

**The rule and the reason.** B-224: *a laboratory declares its work in
countable units — trials, sweep points, tokens, documents — never in minutes.*
A lab that declares twenty minutes has declared a property of the machine it
was written on. Move it to a slower machine and the declaration is wrong; move
it to a faster one and it is wrong the other way; in neither case has anything
about the work changed. And it is wrong *silently* — nothing on the page says
the number was a guess about somebody else's hardware.

**What was built.** `mcf_bench::planned::Work` carries three counts — trials,
arms, tokens — and no fourth field. There is deliberately no duration, no
deadline and no timeout on it, because a type with one would let a laboratory
declare in minutes, and B-224 is a statement about what *may* be declared
rather than about what is usually declared. `checks/tests/work_is_counted_not_timed.rs`
holds that shut: Rust cannot say *no field of this struct means a duration*,
since a `usize` is a `usize` whether it counts trials or seconds, so the check
is on the source of the declaration, its rendering, and the derivation.

**The minutes, derived rather than refused.** Refusing to tell an operator what
a run will cost them is not honesty, it is unhelpfulness with a rule attached.
So the duration is computed: the declared count multiplied by a per-generation
band this machine measured, which makes it an `Estimate` and puts A20's wall
between it and any measurement in the type system. Banded, because a rate has a
spread and a single number is B46's *smallest possible version of a confident
wrong number*. Where two arms have two bands the enclosing one is used — the
faster arm's floor to the slower arm's ceiling — because widening is the
direction an estimate is allowed to be wrong in.

**Measured on this machine.** A run of two SmolLM2 quantizations at 128 tokens
declared and expected:

```
work     at most 200 paired trial(s) across 2 arm(s) — 400 generation(s) of
         128 token(s), 51200 token(s) in all
         expected 1m 6s to 2m 49s at that ceiling — an ESTIMATE from 68
         measured arm(s) of local history, never a measurement and never a
         declaration (B-224, A20)
```

The counts are the same sentence on any machine. The minutes are this
machine's, and say so.

**And absent where there is nothing to derive from** (A7). Asked for the same
two files at ninety-seven tokens, a budget this machine has never used:

```
work     at most 200 paired trial(s) across 2 arm(s) — 400 generation(s) of
         97 token(s), 38800 token(s) in all
         no expected duration: nothing has been measured at 97 tokens here;
         this machine has history at 1, 32, 64, 128, 400, 2000 — two requests
         of different lengths are two different things
```

The declaration still stands, and the absence names what history there *is*, so
the operator can pick a budget that has one. A figure MCF chose instead would
be indistinguishable on the page from one it measured.A behaviour
run gets the same treatment for a different reason: it pins no generation
length by design (D19), so it has no budget to project at.

**B-225 needed no second bookkeeping path.** The register asks that estimates
be scored against actuals and their error tracked, so that a lab whose
estimates are persistently wrong surfaces as a finding. An expectation here is
exactly the per-trial band multiplied by a count both sides agree on, so it
contains the truth if and only if the per-trial band did — which is what
`project::score` already scores, by leaving each measured point out and
projecting it from the others, and what `mcf doctor` already reports (F68: 42
of 49 inside, worst miss 150.1%). A separate score of the derived figure would
be a second number that could disagree with the first about the same history.

## 73 · F73 — A budget proposes, and what it excluded is on the page (B-226, B47, §3.1, A7)

**The failure this prevents.** An operator gives a time budget, the run quietly
does six of the twenty things it would have done, and reports the six. Nothing
on the page is false and the reader is still misled: they are looking at a
sixth of an experiment believing it is the experiment. §3.1's rule is that *ran
6 of 20* always arrives with the fourteen.

**A budget is the operator's, and never the laboratory's.** F72 established
that a lab may not declare its work in minutes. A person may certainly say how
many they have — `mcf bench --within <seconds>` — and the asymmetry is the
point: the constraint comes from outside the apparatus, and what the apparatus
owes in return is a *proposal*.

**`Proposal` has three shapes and no fourth.** `Whole` where everything fits;
`Fewer`, which cannot be constructed without the excluded half, so the fourteen
are in the type and not only in the prose; and `NotEnough`, which is a refusal.
A budget that buys less than two paired trials does not buy a smaller
comparison, because two trials of two arms is the smallest thing that *is* a
paired comparison and producing something below it would be answering a
different question quietly.

**Planned against the slow edge of the measured band.** Against the fast edge a
run would go over budget about as often as under it, which makes a budget
decorative. Under-spending is the harmless direction, so that is the direction
the arithmetic errs in.

**And a budget MCF cannot plan against is a refusal.** Truncating against a
rate it does not have would be inventing the rate (A7); running the full
ceiling anyway would be ignoring what the operator asked for. Measured on this
machine, at a token budget nothing has been run at:

```
mcf: a time budget needs a measured rate to plan against, and there is none
here — nothing has been measured at 45 tokens here; this machine has history
at 1, 32, 64, 97, 128, 400, 2000 — two requests of different lengths are two
different things
  run without --within to take the comparison and give this machine that
  history
```

The refusal names what would fix it, which is the difference between a rule and
an obstacle.

**And a refusal that was arithmetically right for a reason worth reading.**
Asked for sixty seconds at a budget this machine *did* have history at, MCF
refused: *the budget buys less than a comparison*. That looked wrong — sixty
seconds is many generations of a 135M model — until the history was read. It
was right, and why it was right is [F74](#74--f74--the-record-caught-a-third-partys-workload-and-the-projection-swallowed-it-b-217-f71-38-a6-b34).

## 74 · F74 — The record caught a third party's workload, and the projection swallowed it (B-217, F71, §3.8, A6, B34)

**Found while checking something else.** A benchmark of two SmolLM2
quantizations was asked for within sixty seconds and refused: *the budget buys
less than a comparison* (F73). Sixty seconds is a great many generations of a
135M-parameter model, so the refusal looked like a defect. It was not. Read out
of the record, the last three comparisons at those budgets say:

| tokens | competing before | competing after | fastest left trial | slowest |
|---|---|---|---|---|
| 128 | 0.35 core(s) | 0.15 | 400.8 ms | 424.9 ms |
| 128 | 33.05 core(s) | 33.10 | 6.69 s | 13.37 s |
| 97 | 33.70 core(s) | 35.90 | 5.62 s | 18.59 s |
| 97 | 33.25 core(s) | 33.30 | 3.43 s | 18.35 s |

A generation that takes 400 milliseconds on a quiet machine took **thirteen to
eighteen seconds** while thirty-three cores were busy. Planned against the slow
edge of that, sixty seconds genuinely does not buy two paired trials, and the
refusal was arithmetic rather than a bug.

**What was competing was not MCF's.** The processes were a .NET test suite in
an unrelated repository on the same machine, running at 2640% processor and
holding twenty-six cores for the better part of twenty minutes. Nothing about
it is MCF's business except that it was there.

**B-217 worked exactly as built.** F71 said the movement across a run is
recorded as a condition and never as a verdict, and that the reader of such a
record can discard it while MCF cannot discard it for them. That is what
happened: the confound is in the record, in its own field, in parts per
million, for anyone who reads the entry. Without it, the only trace would have
been four unexplained slow numbers.

**And here is the gap it exposed.** `mcf_bench::project::band` reads the
history and does not read the conditions the history was taken under. Four
contended entries sit in it beside sixty-odd quiet ones, and every projection
made from that history — the expected duration on the screen, the band
`mcf explain` shows, the score `mcf doctor` reports — silently rests on them
without saying so. That is not a wrong number; it is a number whose conditions
did not travel with it, which is the thing §3.4 and A6 exist to prevent, and it
is one layer deeper than where MCF was enforcing them.

**What was deliberately not done.** The contended entries were not deleted:
A1 forbids losing them, and they are true measurements of what this machine did
that afternoon. Nor were they filtered out, because filtering needs a threshold
and the threshold is DEC-007's, still open — the same wall F71 stopped at. What
is owed is narrower and needs no decision: **a projection must carry the
conditions of the history it rests on**, so that a reader sees *from 68
measured arms, 4 of which were taken while the machine was not steady* rather
than *from 68 measured arms*. That is registered as B-385.

**A note on how this was found**, because it matters more than the finding.
Nobody was looking for it. A refusal that seemed wrong was checked against the
record instead of being adjusted until it looked right, and the record had the
answer in a field added the previous day for an unrelated reason. That is what
§3.8 is for.

## 75 · F75 — A band that says what it rested on turns a wrong-looking number into a legible one (B-385, F74, §3.4, A6, A7)

**Built to close what [F74](#74--f74--the-record-caught-a-third-partys-workload-and-the-projection-swallowed-it-b-217-f71-38-a6-b34) found.**
`project::band` read this machine's history without reading the conditions the
history was taken under, so four contended entries sat in it beside seventy
quiet ones and every surface rendering a band inherited them in silence.

**A `Point` now carries what else the machine was doing** — thousandths of a
processor, the larger of the readings taken either side of that run, because a
projection should inherit the worse of the two conditions rather than the
flattering one. `band` returns a `Projection`, which is a band *and* a
`Rested`, and the two cannot be separated by a caller: holding only the band is
the defect, so the type no longer offers it.

**Carried, never filtered.** Filtering contended history needs a threshold and
the threshold is DEC-007's, still open — the same wall [F71](#71--f71--the-machine-either-side-of-a-run-is-a-condition-not-a-gate-b-217-dec-007-34-38-a6-a7)
stopped at. Saying what the band rested on needs no threshold at all, which is
why this could be built today and the refusal could not.

**And unknown is not quiet** (A7). Entries written before B-217 recorded
nothing about the machine; that renders as *neither of the two runs it was read
between recorded what the machine was doing, which is unknown and not quiet*,
and a zero there would be a claim MCF cannot support.

**What it looks like on the machine that has the problem.** The same command
that produced F72's clean expectation now says:

```
expected 7m 4s to 89m 8s at that ceiling — an ESTIMATE from 74 measured
arm(s) of local history, never a measurement and never a declaration
(B-224, A20); read between two runs with 33.10 core(s) and 33.10 core(s)
competing
```

Seven minutes to eighty-nine is a useless-looking band, and that is the
finding: it is the *correct* band for this history, and the clause after the
semicolon is the whole reason. Before B-385 the same estimate would have read
*from 74 measured arms of local history* and nothing else — the same numbers,
with no way to tell they came from an afternoon when somebody else's test suite
owned the machine. And `mcf explain`, projecting at a budget whose history is
clean, says the same thing in the other direction: *read between two runs with
0.15 core(s) and 0.15 core(s) competing*.

**The general shape.** Twice now the honest move has been the same one:
measure the condition, attach it to the claim, and leave the judgement to the
decision that has not been made (F71, and this). Neither needed a threshold to
be useful, and both would have needed one to refuse.

## 76 · F76 — A run that reports as it goes shows what moves, not what has not decided (B-227, A4, §3.1, A18)

**Half of B-227 was already true.** A4 keeps what an interrupted run produced:
the pairs are kept, the reason it stopped travels with them, and the comparison
is marked cut short. What was missing is the other half — *reports as it goes*.
A benchmark that takes minutes and says nothing until it finishes is one an
operator cannot tell from a hung one, and one whose forty pairs are first heard
of when it stops.

**Where it goes.** Standard error, and only there. The result of a benchmark is
one thing and goes to standard output; these are the run talking about itself
while it works, and a pipeline reading a verdict must not have to filter
progress out of it. A check counts `println!` against `eprintln!` to keep it
that way — a substring test would forbid the very thing it is asking for, since
one ends in the other.

**What it says, which took a second attempt.** The first version rendered the
verdict, and the verdict while a run is going is *not decided* at nearly every
pair:

```
… after 2 pair(s), so far: not decided after 2 paired trial(s): the arms have
  not separated and the noise is still wider than the difference being looked for
… after 3 pair(s), so far: not decided after 3 paired trial(s): the arms have
  not separated and the noise is still wider than the difference being looked for
```

Forty lines of that is not reporting, it is repeating — and it is worse than
silence, because it buries the one line that will differ. What actually moves
while a run goes on is the two arms, so that is what is shown:

```
… after 2 pair(s), so far: 1894.1 ms against 3511.2 ms
… after 3 pair(s), so far: 1687.1 ms against 1720.3 ms
… after 4 pair(s), so far: 1687.1 ms against 1835.2 ms
… after 5 pair(s), so far: 1687.1 ms against 1720.3 ms
```

An operator watching that sees the second arm settling out of its first cold
reading, which is a thing worth knowing while it happens. Where a verdict *is*
reached the verdict is what is shown, because then it is the news.

**Marked in the line, not by where it appeared.** Every interim line carries
the count it rests on and the words *so far*, so that a reader who scrolls back
cannot take one for the answer. It is not an estimate (A20): it is a real
finding over fewer pairs, which is a different thing and is labelled as the
different thing.

**And it cannot change what the run does** (A18). `so_far` takes the comparison
by shared reference and contains no `break`, no exit and no error path; a check
holds all of that. A benchmark has no pass condition, and a progress line that
could stop a run would be one.

## 77 · F77 — Four outcomes and no total, built before the laboratories that will produce them (B-200, B-201, B40, B41, D2, §3.23, §3.9)

**The failure, in B40's own words.** *A model with no tool-calling that scores
4% on an agentic suite has not been measured badly — it has not been measured.*
The four percent is the wrong instrument's reading, and once it is a number in
a column nothing downstream can tell it from a real one. It sorts. It averages.
It loses a comparison. It becomes a verdict about a model, arrived at by
grading it on a capability it does not have.

**Both rules name `compiler` as their check**, which means the shape has to do
the work. `mcf_core::graded::Graded` has four variants: `Measured` carries a
`Score`, and `NotApplicable`, `Unknown` and `Failed` have nowhere to put one.
The only way a number leaves is `score() -> Option<&Score>` — fallible on
purpose, so that a caller reaching for a number meets the three cases with none
at exactly the point where they were about to flatten them. There is no
`unwrap_or`, no `Default`, no `score_or_zero`.

**`NotApplicable` and `Unknown` are different claims**, and keeping them apart
is the part that would be easiest to lose. *MCF looked and the capability is
absent* and *MCF has not looked* are not the same statement: the second is
resolvable by running a probe and the first is not, and collapsing them would
make a laboratory's silence indistinguishable from a model's limitation (A7).

**`Score` is not a number type.** It carries the laboratory it is on, and
`against` compares two scores only where they share one — returning `None`
otherwise, which is not a failure to compare but the absence of anything to
compare. There is no `PartialOrd`, no `Add`, no `Sum`. Two readings of one
instrument are comparable because that is what an instrument is for; two
readings of different instruments are not, and a type that permitted it would
be an invitation.

**And no total** (B-201). `Profile` holds one outcome per laboratory and offers
no arithmetic across them — no `overall`, no `average`, no `rank`, no `Ord`.
*Which model is better* has no referent once quality is plural, and an
*overall* column is §5's leaderboard wearing local clothes. What a profile does
offer is coverage: `measured by 1 of 3 laboratory(ies); inapplicable to tools`,
because B41 makes coverage travel with every answer and *inapplicable to* is
the half a reader is least likely to be shown.

**Built before the laboratories exist, deliberately.** M6's evaluation labs are
not written yet. The type is the thing that makes the failure unrepresentable,
so it is cheaper — and much more likely to hold — built first than retrofitted
around results that already exist as numbers. A check names each escape hatch
by the string that would introduce it, since what a compiler cannot enforce is
that nobody *adds* the hatch later.

**A note on the check that nearly blocked itself.** The module's prose names
`unwrap_or` and `Default` in order to say they are absent, so a check reading
the whole file failed on the sentence explaining why it passes. It reads the
code without the comments now — the second time this repository has met that
shape, and the rule is the same as it was: a check that blocks the correct work
teaches people to write around it.

## 78 · F78 — Eight Japanese characters cost fifteen tokens here and four there, and the shattering is visible (B-381, PR11, §3.15, F19, A1)

**The observation PR11 came from.** A vocabulary handles text it does not
contain by shattering it, and *where* it shatters is invisible to the person
who wrote the text. A token count answers *how much*; it cannot answer *where*,
and a word that survives whole and a word broken into seven bytes add the same
amount to the same total.

**`mcf segment <model> --prompt <text>`.** No generation, no judgement: it
reads the vocabulary out of the file and shows what that vocabulary does.

```
$ mcf segment SmolLM2-135M-Instruct-Q4_K_M.gguf --prompt "The antidisestablishmentarianism debate"
7 token(s) for 39 character(s) of text, on a vocabulary of 49152 token(s)

  #0        504  "The"
  #1       1598  " ant"
  #2      17889  "idis"
  #3      30834  "establish"
  #4        358  "ment"
  #5      35050  "arianism"
  #6       6866  " debate"

2 of 3 whitespace-separated word(s) survived as a single token; the rest were
broken into pieces. Where a word breaks is a property of this model's
vocabulary and not of the writing, and nothing here rates it (§3.15).
```

**And the same eight characters on two vocabularies.** SmolLM2's byte-level
BPE spends **fifteen** tokens on `日本語のテキスト`, alternating an incomplete
byte with the token that completes the character:

```
  #7      11100  "�"
  #8        224  "テ"
  #9      10391  "�"
  #10       251  "キ"
```

Llama's SentencePiece vocabulary spends **two** tokens on `日本` where
SmolLM2 spends five. Neither is a defect and MCF says so about neither: it is
what these two vocabularies contain, and the reader who is choosing a model for
Japanese now has the fact in front of them rather than a total they cannot
decompose.

**A round trip that is not one is said out loud.** On the Llama vocabulary the
same command reports: *What came back is not what was typed: `"<s> 日本 hello"`
against `"日本 hello"`* — the beginning-of-text token the file asks for and the
space `SentencePiece` was trained with. A surface that quietly stripped those
would be hiding two things the model definitely receives (A1, F19).

**A bug this found, which is the part worth keeping.** The first version took
each token's contribution as `decode(k).strip_prefix(decode(k-1))`, with a
fallback to the whole string when the prefix did not match. On English it was
correct everywhere. On Japanese it reported:

```
  #14       226  "日本語のテキスト"
```

— the last token of the phrase appearing to have produced the entire phrase.
The cause is F19's: a byte-level vocabulary spells one character across
several tokens, so the decode of *k* tokens is **not** the decode of *k-1* with
something appended. The replacement mark standing in for the incomplete
character is *replaced* by the character it stood for, the prefix strip fails,
and `unwrap_or` supplied a confident wrong answer. Comparing **bytes** and
taking what follows the common run is correct in both cases, and the failing
string is now a test.

The general lesson is not about tokenizers. `unwrap_or` on a fallible
derivation is A2's silent failure with a friendly name: the code kept going,
the output looked plausible, and only text in a script the author had not tried
made it visible.

## 79 · F79 — A marker typed into a prompt is shown as what it becomes (B-383, PR11, F37, F26, D46, §3.7)

**The measured cost behind the item.** F37: `<|im_start|>` written into a
prompt reaches the model as ordinary tokens, and the table that produced was
the most decisive-looking wrong answer in this repository. Until now a person
tuning a prompt had strictly *less* visibility than the probe that was fooled
by it.

**`mcf segment` now answers two separate questions about every marker-shaped
thing in the prompt**, and keeping them separate is the whole point:

```
Markers written into the prompt:
  "<|im_start|>" → 7 ordinary token(s). This vocabulary HAS a token spelled
    exactly that, and typed text still does not become it: nothing a person
    writes can produce a control token (D46, F26).
  "<|nope|>" → 6 ordinary token(s). This vocabulary has no such token at all,
    so it is ordinary text here however it is spelled.
  "[INST]" → 3 ordinary token(s). This vocabulary has no such token at all,
    so it is ordinary text here however it is spelled.
  A template whose markers do not survive is a template that does not do what
  it looks like it does (F37, B-383).
```

*Does this vocabulary have such a token* and *does typing it produce one* have
different answers, and the second is always **no**. That is D46 and F26's
safety property, deliberate and load-bearing: a surface that let typed
characters become the token a chat template uses to start a turn would let
anybody forge a turn boundary. So a vocabulary that has `<|im_start|>` and a
prompt that contains `<|im_start|>` still do not meet — and the reader is told
that in a sentence rather than left to infer it from a token count.

**Found by shape, then asked of the vocabulary.** MCF keeps no table of every
family's markers; one would be out of date the week it was written. It notices
`<…>` and `[…]`, bounded in length and stopped by whitespace so that *a < b*
in prose is not a marker, and then asks *this* file — the only authority that
matters.

**Measured here.** On SmolLM2's vocabulary `<|im_start|>` costs seven ordinary
tokens and `[INST]` costs three. Neither does anything. Nothing in the output
says a person was wrong to write them: §3.15's job is to make the effect
visible, not to grade the prompt.

## 80 · F80 — The register's headline was wrong by five items, and the check was green because it skipped what it could not parse (B-383, C5, A1, A2, §3.5)

**Written after the fact, and the delay is itself the finding's second half.**
The commit that fixed this said *F79, F80* and wrote only F79. The number was
then cited four times — twice in this file, once in its own table of defects,
once in the register's changelog — and resolved to nothing for a day, because
the check that resolves identifier citations knew about rules, resolutions,
laboratories, milestones and proposals, and not about findings (F108). What
follows is reconstructed from the commit `d6f1752` and [backlog.md](backlog.md)'s
Version 193, which recorded the measurement at the time.

**What happened.** Adding one row to the register made its headline disagree
with its table. `checks/tests/the_register_counts_itself.rs` exists to stop
exactly that — and it was green.

**Why it was green.** The check skipped any row whose cell count was not what it
expected, instead of failing on it. A row it could not parse was a row it did
not count, silently, and the headline it compared against was therefore a
headline about a subset nobody had named.

**What the skip was hiding**, once it became an assertion:

| row | why it was invisible |
|---|---|
| B-053, B-054, B-057 | two status cells each — the residue of an edit that unblocked them and left the old status behind |
| B-060 | written correctly, with a closure spelled `\|\|`; the check split on the escape |
| the new row | written that day |

**The measurement.** The register said **273 items** and held **278**.
Recomputed from the table: 223 build items — 111 done, 1 dropped, 13 in
progress, 36 blocked, 62 open.

**The fix.** An unparseable row is an error, and the parser honours `\|` as a
literal pipe. The counts are recomputed from the table rather than compared with
a number somebody typed.

**The shape, which was the third of it here at the time and the fourth by
[F81](#81--f81--korean-costs-65-times-english-on-one-vocabulary-and-41-on-another-and-neither-is-a-fact-about-korean-b-379-315-dec-002).**
*A check that passes while failing to do its job.* The failure mode of a check
is not a wrong answer — it is **no answer, reported as a right one**. F102, F103,
F105 and F106 are the same thing in four other places, and every one of them was
green when it was wrong.

## 81 · F81 — Korean costs 6.5 times English on one vocabulary and 4.1 on another, and neither is a fact about Korean (B-379, §3.15, DEC-002)

**`mcf explain` now answers *what does each language cost here*.** One
sentence — the first clause of Article 1 of the Universal Declaration of Human
Rights, in the United Nations' own translations — put through the model's own
vocabulary, in thirteen languages spanning Latin, Cyrillic, Greek, Han, Kana,
Hangul, Arabic and Devanagari.

On SmolLM2-135M-Instruct:

| language | tokens | characters | against the cheapest |
|---|---|---|---|
| English | 13 | 63 | 1.0x |
| German | 24 | 64 | 1.8x |
| Chinese (Simplified) | 29 | 19 | 2.2x |
| Arabic | 45 | 51 | 3.4x |
| Russian | 54 | 69 | 4.1x |
| Greek | 70 | 83 | 5.3x |
| Japanese | 72 | 42 | 5.5x |
| Korean | 85 | 39 | 6.5x |
| Hindi | 92 | 87 | 7.0x |

**And on the Llama vocabulary of `stories15M`, the order changes.** Russian
costs 54 tokens on SmolLM2 and **23** there; Greek costs 70 on SmolLM2 and
**84** there. Neither vocabulary dominates, which is the result that makes the
point: this is not a ranking of languages by difficulty. It is a table of what
somebody happened to put in a file.

**The ratio is over the same meaning, not the same character count.** The
register asked for *tokens per character*, and per-character answers the wrong
question: Chinese writes this sentence in nineteen characters, so it looks
expensive per character while costing fewer tokens outright than French. What
compounds — context, money, time — is the total for the same meaning, and
these sentences *are* the same meaning by construction, which is the whole
reason for using a carefully translated parallel text rather than one MCF
wrote. The character count stays on every line so that a reader who wants the
other reading can have it (A1).

**The wording is the hard part, and it has its own check.** The arithmetic is
counting tokens. What is easy to get wrong is that *expensive* reads as *bad*:
a sentence letting a vocabulary's spelling be heard as a judgement about a
language, or about how well the model speaks it, would be the most damaging
thing in this repository and the easiest to write by accident. So
`checks/tests/a_language_cost_is_about_the_vocabulary.rs` requires the answer
to name what it is a property *of* — the file — to deny what it is not, and to
contain none of a list of grading words. A model can be excellent at a language
its vocabulary spells expensively.

**A smaller thing worth recording.** The check first failed against a sentence
that is in the source: `rustfmt` had broken it across lines with a trailing
`\`, so a substring search found nothing and would have gone on asserting
about a string that never appears. It joins continuations first now. That is
the fourth variant this repository has met of *a check that passes without
checking* (F80), and the second where the fix was to read the source the way a
compiler does rather than the way a text editor shows it.

## 82 · F82 — A prompt's cost can be stated before it is sent, and the measured context is on a terminal and nowhere else (B-382, B-386, A21, A1, F42)

**The question, answered before anything is sent.** Is this guidance document,
this transcript, this file too long for this model? The same text is a rounding
error on one model's window and does not fit at all on another's, and there was
no surface that said so. `mcf segment` now does:

```
1185 token(s) of prompt against a DECLARED context of 128 token(s) — 925.7%
of it: this prompt does not fit, before a single token of answer.
```

and, where it does fit, what is left:

```
8 token(s) of prompt against a DECLARED context of 8192 token(s) — 0.0% of
it, leaving 8184 token(s) for everything else — the answer, and anything else
in the window.
```

*Leaving N for everything else* rather than only a percentage, because a share
of the window hides that the answer needs room in it too.

**Declared, and said to be declared** (A21). The number is the file's claim
about itself, and F42 measured that an engine on a real machine can take fewer.
So the word is in capitals, and the sentence names `mcf probe` as what would
verify it — a marking with no route attached is a disclaimer.

**Which is where the item stops, and why.** B-382 asks for the count against
the **usable** context *measured* for this model. That measurement exists —
F42 took it — and it is **not in the record**. `mcf probe` prints its findings
and writes only `--apply`'s configuration change, so the usable context this
machine established for a model lives on a terminal that has since scrolled.

**A1's plainest case.** *A measurement nobody can find later is the same as one
not taken.* Every surface that wants a measured figure rather than a declared
one is blocked behind this, and the fix is not in B-382's scope: it is that a
probe's outcome should be written down. That is B-386, opened here, and B-382
stays in progress until it exists. Building a reader for a record that is never
written would be dead code pretending to be a feature.

**No declaration is unknown, not unlimited** (A7). A file that declares no
context length gets *there is nothing to state it against — which is unknown
rather than unlimited*, and a declared zero is treated as no declaration rather
than as a division.

## 83 · F83 — The probe writes it down, and the prompt is measured against what the machine takes (B-386, B-382, A1, A9, D42, F42)

**Closing [F82](#82--f82--a-prompts-cost-can-be-stated-before-it-is-sent-and-the-measured-context-is-on-a-terminal-and-nowhere-else-b-382-b-386-a21-a1-f42)'s
gap the day it was found.** A probe measured a model's usable context, printed
it, and wrote nothing. `EntryKind::ModelProbed` now exists and `mcf probe`
writes what it observed — the declared figure, the accepted one, the engine,
and the engine's own words where it refused, because a refusal for an unrelated
reason would otherwise be read back as a short context.

**Recorded whichever way it came out** (A9). *Agrees* is as much a measurement
as *diverges*, and a record that kept only the surprising half could not answer
*what does this machine take*. A check requires the write to sit after both
branches rather than inside the divergence one.

**Distinct from the act it might lead to** (D42, D43). `ModelProbed` is an
observation; `ModelConfigured` is somebody deciding to address a model
differently. A probe that changes nothing still measured something, and
collapsing the two would make *MCF looked* and *MCF changed* the same entry.

**The loop, end to end on this machine.** `mcf probe` on `stories15M-q8_0`:

```
 declared 128 token(s)
 accepted 127 token(s) of prompt, with one left to generate
 agrees the file's claim holds
 recorded in /home/gauge/.local/share/mcf/record.jsonl
```

and then, with no probe re-run and nothing passed between them:

```
$ mcf segment stories15M-q8_0.gguf --prompt "Once upon a time there was a small brave mouse."
12 token(s) of prompt against a MEASURED context of 127 token(s) — 9.4% of it,
and the file declares 128. Measured is what `mcf probe` found this engine on
this machine actually accepts, which is the number a prompt has to fit
(B-055, F42, §3.4).
```

**The declaration stays in the sentence.** A measurement supersedes a claim for
the purpose of deciding whether a prompt fits — that is what taking one is for
(A21) — but *this file claims 128 and this machine takes 127* is itself the
finding, and dropping the claim would hide that the two can disagree. On a
model where they diverge, that clause is the whole story.

**And the reader will not answer about a different file.** A context measured
for one artifact is not a fact about a differently quantized sibling; the path
must match exactly, because the conditions §3.4 requires include which artifact
was asked.

## 84 · F84 — The branch is read from the attribution, and reading it from the category is the obvious wrong design (B-233, B24, §7.10, §3.4)

**B-233's requirement.** *Environment failures are a distinct taxonomy branch
from model failures: an out-of-memory from competition is a condition of the
run, never the model giving up.* Recorded as the model's, a busy afternoon
becomes a claim about a model — and nothing downstream can undo it, because a
wrong attribution reads exactly like a right one.

**The first implementation was wrong, and it was the obvious one.**
`Category::branch()`: a hundred and eleven categories, each mapped to one of
four branches, derived from its domain with a handful of stated exceptions. It
compiled, it was exhaustive, and running it against every `Failure::new` in the
workspace produced eleven disagreements — all of which were the *map* being
wrong and the code being right:

- `probe.inconclusive` attributed to the **machine**, because the machine
  misbehaved. The map said MCF's.
- `engine.unavailable` attributed to **MCF**, because MCF's own stand-in does
  not implement that format. The map said the environment's.
- `config.invalid` attributed to the **user**. The map said MCF's.
- `engine.exit.immediate` attributed to the **machine**. The map said the
  artifact's.

**A category says *what went wrong*. Only the attribution says *whose*.** The
same category is honestly attributable to different parties depending on the
situation, which is precisely why `Attribution` is a separate axis that
`Failure::new` demands and gives no default for. So B-233 was **already
structurally satisfied** before any of this: every failure classifies
unambiguously, because every failure supplies the axis that classifies it.

What is now added is the coarser reading a surface needs — `Attribution::branch`
— and what is checked is only what is not structural: that the model's branch
is reachable *only* through `ModelUnderTest`, that every branch is reachable at
all, and that no construction in the workspace attributes a model's behaviour
elsewhere or a condition of the run to the model. That last check passes across
the whole workspace with no exceptions.

**`Unattributable` returns `None`, not a fourth branch.** B24 makes *MCF cannot
tell* a real result, and folding it into a branch would be the attribution MCF
refused to make, made anyway.

**The general lesson.** The wrong design was more code, more precise-looking,
and produced a table of eleven violations that would have been "fixed" by
editing eleven correct call sites. A check that disagrees with working code is
evidence about the check first. This is the second time in this session that
running a new rule against the existing codebase is what showed the rule was
wrong — the first being the register's parser (F80), where the code was right
and the reader was not.

## 85 · F85 — An idle daemon took zero processor ticks and issued zero reads in ninety seconds (B-187, B-108, B4, D5, §3.13, §6.18)

**Two items, one measurement.** B-187: *counter reads are zero outside a lab
run.* B-108: *the benchmark subsystem consumes nothing during ordinary
serving.* Both were open and both are the same discipline seen from two sides.

**Measured on this machine.** A running `mcf serve`, with nothing asked of it,
read from `/proc` before and after ninety seconds:

```
ticks:  336 -> 336   (0 of 100/s over 90 s)
syscr:  290824 -> 290824
rchar:  2294781810 -> 2294781810
```

Not *small*. **Zero** processor time and **zero** read syscalls — the process
did not execute. That is the difference between a daemon that polls quietly and
one that genuinely waits, and it is only visible because the counters are
integers that either moved or did not.

**Why it matters beyond tidiness.** A daemon that polls thermal counters to
look responsive is a daemon that is one of the competitors it reports (§3.8) —
which [F74](#74--f74--the-record-caught-a-third-partys-workload-and-the-projection-swallowed-it-b-217-f71-38-a6-b34)
showed is not hypothetical: something *was* stealing this machine, and MCF's
value there depended on not being part of the problem. And a benchmark
subsystem with an idle cost makes every serving measurement conditional on
whether it was compiled in.

**What holds it.** B4's discipline — a reading exists because somebody asked
for it, never because a clock came round — plus one structural fact:
`mcf-serve` does not depend on `mcf-bench`. The daemon cannot start a benchmark
because it cannot name one, which is a stronger guarantee than any measurement
of its idle cost. `checks/tests/idle_mcf_reads_nothing.rs` pins all three:
no spawned loop in the serving path, no reader of the machine outside a command
the operator ran, and no path from the daemon to the benchmark crate.

**The one counter MCF does read** is a GPU temperature through NVML, inside
`Machine::read_through`, reached only from `mcf doctor`, `mcf run`'s fitment
check, `mcf pull`'s, and `mcf verify`. Every one of those is something a person
typed.

## 86 · F86 — A field of one is refused by name, and a foreign number has no route in (B-167, B-127, B34, B43, §6.23, §5)

**Built ahead of the recommender, for the reason [F77](#77--f77--four-outcomes-and-no-total-built-before-the-laboratories-that-will-produce-them-b-200-b-201-b40-b41-d2-323-39)
gives.** Both items specify a *type-level* property, and a type is cheapest to
get right before there are values to retrofit it around.

**B-167: no foreign number reaches a recommendation.** `Candidate` holds a
`LocallyMeasured<Profile>` and there is no other constructor. A contributed
measurement is a `FromCorpus`, `origin` offers no conversion in either
direction, and so a corpus number cannot arrive by any route — not by being
confirmed, not by being averaged in, not by being passed as an argument that
happens to typecheck.

The failure mode this guards is not somebody deliberately ranking on foreign
data. It is a number arriving through three layers of helpers with nobody
noticing where it came from. A type that has to be *written* at the boundary is
the only thing that survives that, which is why `LocallyMeasured::new` is
deliberately not a `From`.

**B-127: a field of one is not a field.** `Field::ordered_by` returns a named
refusal rather than a list:

```
one candidate (a) is not a field: a frontier with a single point is not a
frontier, and *the best of one* recommends whatever it was handed (§6.23)
```

and it names the one, because the operator's next move is to name a second. An
empty field is a *different* refusal from a field of one, since *nothing was
considered* and *only this was considered* are different situations.

**And the case that would otherwise slip through.** A field of six where one
candidate has a reading is a field of one wearing six names — the other five
were not measured badly, they were not measured (B40). `TooFewMeasured` says
so, and names the laboratory and the count. Asking a laboratory nobody ran
gets the same refusal rather than an empty ranking, because an empty list reads
as *nothing is any good* rather than *nothing was asked*.

**And the coverage is computed, not stored** (B-202). `Field::coverage` says
which laboratory the ordering was made on, how many of the candidates it
measured against how many were considered, which laboratories some candidate
was verified inapplicable to, and which informed nothing at all:

```
on agentic alone, which measured 2 of 3 candidate(s); inapplicable to tools
(verified absent, which is not a low score — B40); vision informed nothing
here, which is *not run* rather than *no difference* (A7)
```

*Two of three* rather than *two*, because the bare count hides the candidate
that was not measured. Computed from the same field the ordering came from, so
a ranking cannot end up beside coverage describing a different set of
candidates. And a laboratory with one reading is not *silent*: what a reader
needs from that word is *nothing came from here at all*.

**Unmeasured candidates are absent from the ordering, not last.** A missing
reading is not a low one, which is [F77](#77--f77--four-outcomes-and-no-total-built-before-the-laboratories-that-will-produce-them-b-200-b-201-b40-b41-d2-323-39)'s
rule showing up one layer higher — and ordering is stable, so two candidates a
laboratory could not tell apart keep the order they were considered in rather
than one the sort invented.

## 87 · F87 — A contribution has nowhere to put a task, and no way to be unsent (B-171, B-203, B-251, B-310, B42, B54, D21, §6.30, §3.20)

**Four items, one shape, built ahead of M9** for the reason F77 and F86 give:
each specifies a property of the format, and a format is cheapest to get right
before there is anything in it.

**Outcomes, never artifacts** (B-171). A contribution that *strips* task
content on the way out is one line away from not stripping it, and the line
lives in the export path where nobody looks twice. A contribution with nowhere
to *put* task content cannot leak it however the export is written. There is no
field here that can hold a prompt, a completion, a document or a path, and a
check forbids each by name.

The stakes are not only privacy. A benchmark task that travels ends up in
somebody's training data, and a corpus that leaks its own tasks measures
memorization from then on.

**Comparisons in preference to absolutes** (B-251, B54, §3.27). *This arm was
12% quicker than that one over forty pairs* survives travel: both arms met the
same afternoon, so what differs between them is the arm. *This took 380 ms*
does not: it is a fact about somebody else's hardware that the reader cannot
scale. So `Absolute::new` is **fallible** — it refuses unless all thirteen of
the condition floor's questions are answered — while a `Comparison` needs the
conditions it was taken under and not a complete floor. The preference is in
what is easy to construct, not in a docstring.

**A custom workload is refused, and refused early** (B-203, B42, A25). The
marking is on the row from production. A marking applied at export is a marking
that can be forgotten at export, in the one place under time pressure with the
operator watching a progress bar. Both routes in refuse it, and a complete
condition set does not rescue it — the objection is not that the conditions are
unknown, it is that nobody else has the workload.

**And nothing retracts** (B-310, B63, D21). There is no `retract`, `unsend`,
`withdraw`, `recall` or `revoke`, and a check forbids all five by name so that
adding one is a deliberate act against a test rather than a helpful-looking
commit. Once something has left, it has left; an affordance suggesting
otherwise would be the most consequential false promise MCF could make, because
a person would rely on it. What exists instead is the terms, which say so
*before* anything is sent:

```
What leaves is outcomes only: scores, classifications, conditions and effect
sizes. No prompt, no completion, no task, no fixture and no file leaves —
there is nowhere in the format to put one. Publication cannot be undone: MCF
offers no retraction, because there is no such act (D21, B63).
```

**And the other direction: what arrives** (B-166, B-172). An import is a
claim. `Imported` holds the figure as a `FromCorpus`, which cannot back a
recommendation (B-167) and cannot render as MCF's own, and it says so in
words — *DECLARED elsewhere … `mcf probe` and `mcf bench` are what would make
it a measurement here*. There is no `verify`, no `promote`, no `into_local`:
verification does not convert a claim, it **replaces** it, exactly as A20
replaces an estimate.

`Reproduction` has three states and all three are outcomes. `Measured` keeps
*both* figures side by side — only the local one throws away the comparison,
only the difference throws away what was compared. `WillNotFitHere` is a
complete answer rather than a refusal to answer: *it needs 48 GiB and this
machine has 24* is what the operator asked. And `NotAttempted` is its own
state, because unattempted is not agreement.

A divergence between two machines running one identifier is the most valuable
thing a corpus can learn about itself — how far a result travels — and
recording it as an error would throw away the one observation nobody else is
positioned to make (§6.29).

**A small recurrence.** Those terms name every forbidden thing in order to say
it is absent, so the check that forbids them failed on the sentence explaining
why it passes — the third time in this session (F81 was the second). The check
skips the terms now. The pattern is stable enough to name: *a rule and the
prose describing the rule cannot be distinguished by substring search*, and a
check that does not account for it blocks the correct work.

## 88 · F88 — A behaviour laboratory's bound has nowhere to put a wall clock (B-230, B-223, B45, D8, D13, §3.8)

**B-230, and it is [F72](#72--f72--work-is-counted-minutes-are-derived-banded-and-sometimes-absent-b-224-b-225-b46-d14-a20-a7)'s
argument one level up.** F72 established that a laboratory declares its *work*
in countable units. This says the same of its *bound*: a behaviour laboratory's
deadline is a token budget, never a wall clock.

**The specific harm.** A behaviour run bounded by minutes gives a model on a
busy machine fewer attempts than the same model on a quiet one. The result is
supposed to be about the model and becomes partly about the afternoon — and it
happens *silently*: the run completes, reports fewer outcomes, and nothing on
the page says the machine is why. A token budget counts the same everywhere.

**A timing laboratory is the opposite case** and keeps its wall clock, because
elapsed time is its whole subject: a timing run that will not finish is a
measurement about this machine (§3.8), which is what was being asked. So
`Bound` has exactly two variants, one of which carries a `Duration`, and a
check pins the count at one — a second would be the forbidden wall clock
wearing a different name.

**B-223 in the same constructor.** `Planned::new` takes a `Calibrated` and
there is no `Default`, no second constructor, and no `Option` around it. B45's
tier ordering — calibration precedes measurement — becomes a property of the
type rather than a convention, and the difference matters because a convention
is what somebody skips at four in the afternoon. An evaluation on a
configuration nobody calibrated is a measurement of an arbitrary sampling
setting wearing a model's name.

**One type for both classes, rather than two.** The refusal is a `Result` from
the constructor rather than two separate types that cannot express each other's
bound. Two types would be stricter on paper and would drift: every feature
added to one has to be added to the other, and the second copy is where the
rule quietly stops being enforced.

## 89 · F89 — A figure with a unit and nothing behind it is the most convincing kind of wrong (B-188, B-163, B-164, B39, B31, A20, A7)

**B39's violation, stated exactly.** A platform with no power interface yields
a number derived from processor utilization. Multiply a percentage by a
nameplate wattage and you have a figure with the right unit, a plausible
magnitude, and no measurement in it at all. It is more dangerous than an
obviously wrong number because nothing about it looks wrong.

`Energy` has three variants — `Measured`, `Modelled`, `Unknown` — and the only
way joules leave is `measured_millijoules() -> Option<u64>`, fallible so that a
caller reaching for a number meets the other two where they were about to
flatten them. There is no `unwrap_or`, no `Default`, no `millijoules_or_zero`,
and the word *utilization* appears exactly once in the module: inside the
sentence that refuses it. A check pins the count at one, because a second
occurrence would mean it had become an implementation rather than a warning.

**The sampling rate is in the type** (B-188, §3.4). Energy read at one hertz
across a two-second run has seen two samples, and what it missed is most of the
run. A reading whose rate does not travel cannot be compared with one taken at
another rate, and nobody can tell that from the number — so `comparable_with`
requires the same rate *and* the same counter, since a package-level figure is
not a device-level one. A modelled figure compares with nothing at all, not
even another model: two models are two authors' opinions rather than two
readings.

**And what was watching** (B-163, B-164). The condition floor has carried an
`instrumentation` entry as free text since it was written, which is enough to
*note* a profile and not enough to *refuse* on one. `Timed::new` is fallible
and refuses a `Profile::Deep`: a timing taken while a profiler was attached is
a timing of the profiler as much as of the model.

**`Light` is admitted, and that is the interesting case.** It carries a
measured residual — how much the watcher moved the measurement, characterized
against the same run unwatched — so the perturbation is a condition and A6 lets
the measurement travel with it. `Deep` is refused not because it is worse but
because there is *no single residual to carry*: tracing overhead depends on
what the model did, so it is a distribution rather than a number.

**No threshold anywhere.** B31 asks that the overhead be characterized, not
that it be small. A cutoff here would be MCF deciding how much perturbation is
acceptable for somebody else's measurement — the same kind of figure DEC-007
exists to derive rather than assume, and the same wall
[F71](#71--f71--the-machine-either-side-of-a-run-is-a-condition-not-a-gate-b-217-dec-007-34-38-a6-a7)
stopped at. A check forbids the words.

## 90 · F90 — The contention instrument reported 35 cores on a 32-thread machine, because it divided by the window it meant to use (B-216, B-217, DEC-007, A2, §3.8)

**Found by an operator's question, not by a test.** Asked whether the noisy
benchmarks of [F74](#74--f74--the-record-caught-a-third-partys-workload-and-the-projection-swallowed-it-b-217-f71-38-a6-b34)
were simply a machine with headroom to spare, the first thing to check was the
core count. This machine is a 16-core, 32-thread Ryzen 9 9950X. The record
contained a comparison whose conditions said **44.05 cores competing** — a
figure the machine cannot physically produce.

**The instrument was dividing by the interval it intended to wait.**
`contention::sample` reads every process's accumulated processor time, sleeps
`OVER` (200 ms), reads again, and converts the difference to a rate by dividing
by `OVER`. But walking `/proc` costs real time — one file read per process —
and that time falls *inside* the window. The true interval is `OVER` plus two
walks, and the reported rate is inflated by exactly their ratio.

**It inflates most when it matters most.** The walk is slower when the machine
is busy, which is precisely when the reading is being taken and relied upon.
Measured against `/proc/stat`, which is the kernel's own accounting and cannot
exceed the core count:

| condition | dividing by `OVER` | dividing by the elapsed window | `/proc/stat` |
|---|---|---|---|
| quiet | 1.25 cores | 1.16 | 1.16 |
| 48 spinners on 32 threads | **35.20** | 28.65 | 28.89 |
| 48 spinners, repeat | 35.55 | 29.19 | 29.40 |
| 48 spinners, repeat | 35.20 | 28.81 | 29.10 |

Eight percent over when quiet; **twenty-two percent over under load**, and past
the physical ceiling. The per-process summation itself was never wrong — with
the measured interval it agrees with the kernel to under one percent.

**Fixed, and the interval is now a condition.** The window is measured with the
monotonic clock and carried on the `Snapshot` as `over_millis`, because it is
not constant: it grows with the number of processes and with how busy the
machine is, and two snapshots taken over different windows are not the same
measurement (§3.4). Rebuilt and re-measured under the same 48 spinners, MCF
reports 28.5–29.3 cores where the kernel reports 32.0–32.2 — under rather than
over, and never above the ceiling.

**Two checks now hold it.** One forbids the old arithmetic by name. The other
asserts that no snapshot reports more cores than the machine has, with ten
percent of slack for tick granularity — an assertion the defect would have
failed by more than twice that margin.

**What this does to what was already recorded.** Every machine reading in the
record before this fix is high by roughly a fifth. The recorded *44.05 cores*
was about 36; the recorded *33* was about 27. The entries stay as they are
(A1) — they are what the instrument said — and the conclusion of F74 is
unchanged and in fact sharpened: those runs were at 85 % to 113 % of what a
32-thread machine can deliver. There was no headroom at all.

**And a caution about the class of defect.** The reading was wrong in the
direction that makes a machine look busier than it is, which is the direction
that would have made a refusal threshold fire too often. Had B-217's refusal
been built when it was asked for, it would have been calibrated against an
instrument that was over-reading by a fifth under exactly the conditions the
threshold governs. Two of this session's decisions not to invent a threshold —
[F71](#71--f71--the-machine-either-side-of-a-run-is-a-condition-not-a-gate-b-217-dec-007-34-38-a6-a7)
and [F75](#75--f75--a-band-that-says-what-it-rested-on-turns-a-wrong-looking-number-into-a-legible-one-b-385-f74-34-a6-a7)
— turn out to have been protecting against this without knowing it. *Measure
the instrument before you calibrate anything against it* is §6.16 with a number
attached.

## 91 · F91 — The sensors were there the whole time, one directory across (B-084, DEC-007, A7, A2, §3.4)

**A recorded finding was wrong, and it had closed a question.** F53 concluded
that *the only sensor this machine exposes reads sixteen degrees, which is not
a processor temperature*, and DEC-007's thermal half has been blocked on that
ever since. It was reading `/sys/class/thermal`, where this board publishes one
ACPI zone that does read 16.8 °C. One directory across, `/sys/class/hwmon`
carries `k10temp`:

```
k10temp/Tccd1 82.8 °C (processor die)
k10temp/Tccd2 77.5 °C (processor die)
k10temp/Tctl  87.7 °C (processor package)
acpitz        16.8 °C (board)          ← the sensor F53 found
```

Sixteen degrees against eighty-eight. **The operator was right that the
real-time information was accessible somewhere**, and a wrong finding is worse
than an open question: an open question gets revisited.

**MCF was reading no processor temperature at all.** The `thermal_state`
condition reported accelerator temperatures and nothing else, so every
measurement in the record carries no trace of how hot the thing doing the work
was — which is exactly the half of the condition DEC-007's open question is
about. It now reads: `thermal_state=processor k10temp/Tccd1 66.8 °C (processor
die), critical point not published by this chip, accel#0 30 °C`.

**Everything is reported and nothing is chosen.** A machine has many
thermometers measuring different things. MCF reads all of them, labels each
with the chip that published it, classifies the ones it recognises, and neither
averages them nor picks a headline. A die reading is preferred to a package one
where both exist, because a control temperature can carry a vendor offset —
even when the die reads *lower*.

**A second sentinel, caught by the same discipline as [F90](#90--f90--the-contention-instrument-reported-35-cores-on-a-32-thread-machine-because-it-divided-by-the-window-it-meant-to-use-b-216-b-217-dec-007-a2-38).**
An NVMe drive publishes `temp2_max` as `65261850`, which rendered as *critical
at 65261.8 °C* beside real limits. A number no thermometer produced, presented
with the confidence of one that was measured. A limit outside −50 °C to 200 °C
is a chip's way of saying nothing, and absent is what it means (A7).

**The critical point is often absent entirely, and that is not a default.**
Intel's `coretemp` publishes one; the `k10temp` here publishes none. So MCF
cannot compute *how close to throttling* universally, and does not pretend to:
the rendering says *critical point not published by this chip*, because a
reader who is not told the limit is missing will assume there is headroom.

**Occupancy has the same shape.** AMD publishes `gpu_busy_percent` in sysfs —
readable by any process. NVIDIA publishes nothing there and requires NVML.
Intel's `i915` derives occupancy from perf counters needing privilege. So a
card MCF cannot poll reports `unknown` **with the reason**, never zero: an
operator must be able to tell *nothing was competing* from *MCF could not see*.

**And the table problem, answered properly.** Classifying sensors means a table
of driver names, and this repository has already said tables go stale the week
they are written (F79). The honest answer is not a longer list but a route:
`mcf support --into <path>` writes what a maintainer would need — driver names,
kernel, architecture, every sensor with its label and reading, and a section
naming exactly what MCF could not account for. It is a **file**, not an upload;
nothing here contacts anybody, which is the simplest way to satisfy §3.20's
gate. It carries no prompt, no model output and no file content, on the same
reasoning as `contribution` (B-171, A25). And the offer appears where the gap
is found — `mcf doctor` says so — because a gap nobody is told about is a gap
nobody reports.

On this machine it correctly names `r8169_0_f00:00`, a network chip this build
does not classify, and `nvidia`, whose occupancy sysfs does not carry.

**Windows, honestly.** `MSAcpi_ThermalZoneTemperature` through WMI is the same
ACPI zone that reads 16.8 °C here, and is frequently absent. The real per-die
registers need a kernel driver, which MCF does not ship. A Windows build should
expect `Unknown` for the processor and say so, rather than substituting a board
sensor for a die one — which is precisely the mistake this finding corrects.

## 92 · F92 — The headline number had no measure of itself, and the sentence beside it claimed otherwise (B46, B54, A6, §6.16, §3.27)

**Every benchmark ended in this sentence:**

> *the right arm is quicker by **356.0%**, after 6 paired trial(s) — noise alone
> produced a gap that big **3.1%** of the time*

Two numbers, the second looking like it qualifies the first. It does not.
`one_sided_luck` is an exact sign test and its entire input is the count of
positive and negative differences:

```rust
let ahead  = differences.iter().filter(|held| **held > 0).count();
let behind = differences.iter().filter(|held| **held < 0).count();
```

Magnitudes are discarded before the statistic is computed. So *3.1%* answers
*did the right arm win more often than a coin would* — correctly, exactly, in
whole numbers — and says nothing whatever about **356.0%**. The clause *"noise
alone produced a gap that big"* asserted that it did. That sentence was false,
and it was on the front of every comparison MCF has ever produced.

**How blind it was, measured.** Every comparison on this machine carrying a
machine reading:

| competing | n | reported | luck | what the pairs spanned |
|---|---|---|---|---|
| 0.15 c | 6 | by 13.9 % | 3.12 % | 15 points |
| 0.35 c | 6 | by 133.2 % | 3.12 % | 23 points |
| 33.05 c | 6 | by 356.0 % | 3.12 % | **1040 points** |
| 33.25 c | 12 | by 206.5 % | 3.86 % | 954 points |
| 44.05 c | 9 | by 114.0 % | 3.91 % | 616 points |

The confidence figure is pinned near three percent across evidence differing in
quality by a factor of **sixty-nine**, because 3.125 % is simply 2/2⁶ — what six
unanimous pairs give regardless of what they contain. And each magnitude
carried a decimal place: *356.0 %* is a claim of one part in a thousand made
from six numbers spanning tenfold.

**The inconsistency this exposes.** B46 has always said that a number predicted
from a rate is a range and that one number is *the smallest possible version of
a confident wrong number*. Every **estimated** figure in MCF is banded — the
projection, the expected duration, `Estimate` itself. The one **measured**
figure, the headline the whole benchmark product exists to produce, was a bare
point.

**Fixed with the machinery already present.** The bounds are order statistics of
the paired differences: sorted, the *k*-th and *(n+1−k)*-th bracket the median
with a probability that is a binomial tail, which `binomial_tail` already
computed for the sign test. Exact, integer, no resampling, no floats, and no
distributional assumption beyond the exchangeability the interleaving exists to
provide. The same records now read:

| competing | was | is |
|---|---|---|
| 0.35 c | by 133.2 % | **128.0 % to 150.7 %** (96.8 %) |
| 44.05 c | by 114.0 % | 33.4 % to 181.9 % (96.0 %) |
| 33.70 c | by 102.3 % | **4.1 % to 349.4 %** (96.8 %) |
| 33.05 c | by 356.0 % | **118.6 % to 1158.8 %** (96.8 %) |

The quiet run's interval is 23 points wide — a real measurement. The 33.70 c run
reported *102.3 %* from evidence consistent with **4.1 %**, very nearly nothing
at all. Nothing on the page had distinguished them.

**The coverage is reported, not aimed at.** Six pairs cannot express 95 %; the
widest interval available covers 96.875 %. Saying so is the same discipline as
reporting the count a run actually needed rather than the one it hoped for
(F53).

**A third verdict, because the two claims are different.** When the interval
runs from below the caller's resolution to above it, the order is established
and the size is not. `Verdict::Ordered` says both:

> *the left arm is quicker — noise alone put them in this order 0.1 % of the
> time after 60 paired trial(s). HOW MUCH quicker is NOT established at the
> 5.0 % you asked about: the evidence spans 0.0 % to 12.0 %. The order is a
> result; the size is not, and this comparison is not fit to contribute*

A variant rather than a flag, so nothing downstream can render it as a
measurement by forgetting to check a boolean — and MCF invents no threshold,
because the resolution came from the caller.

**What the change caught in MCF's own tests.** Three existing tests encoded the
old, weaker claim and failed:

- A six-percent effect under fifteen-percent noise at **sixty pairs** had been
  reported as *differ by 6 %*. Its interval reaches zero. Sixty pairs settle the
  order and do not settle the size, and now say so.
- Eight wins and four ties had been *differ*. A third of the pairs showed
  nothing, so the interval on the median reaches zero — the point median of
  10 % had been standing in for evidence that was not there.

Neither was a regression. Both were the point estimate having concealed how
little it rested on.

**And one place MCF cannot do this yet.** The interval is an order statistic of
*paired* differences. For arms assembled from separate sessions there are no
pairs, and the two-sample equivalent needs a rank-sum distribution this crate
does not have. `Verdict::Apart` therefore reports a point and **says that it is
one** — a distinct variant, so nothing can mistake it for a paired interval.
Inventing a range from the two arms' own ranges would be exactly the confident
wrong number the whole finding is about. That is B-388.

**Recomputed on read, and the record untouched** (B55, B56, A1). The trials are
kept, so every comparison already written renders with the interval its own
pairs always supported. Nothing on disk was rewritten; the summary is derived
each time it is asked for, which is the rule that made this recoverable at all.
The oldest entries in this machine's record gained their intervals without a
byte changing.

## 93 · F93 — Every measurement MCF has taken is attributed to an instrument it cannot identify (§3.4, A1, A2, A7, §6.16)

**The premise.** §3.4 makes MCF's own version part of the condition set of
every measurement, because a number is only interpretable if you know what took
it. The instrument is the first condition.

**What the condition actually said.** All 52 comparisons in this machine's
record, without exception:

```json
{ "version": "0.1.0-m0", "revision": null,
   "rustc": "rustc 1.98.0", "profile": "release",
   "target": "x86_64-unknown-linux-gnu" }
```

Identical. And they span one day during which three measuring instruments
changed:

| corrected | instrument | what it did to readings |
|---|---|---|
| 21:39 UTC | contention (F90) | competing-processor readings **~22 % high** under load |
| 22:27 UTC | thermal (F91) | **no processor temperature at all** |
| 23:02 UTC | effect size (F92) | headline was a **point estimate** with no measure of itself |

Nothing in any of the 52 records says which side of any of those three it falls
on. A comparison taken at 07:00 and one taken at 22:00 are, as far as the
condition set is concerned, the work of the same instrument.

**The mechanism existed and had never fired.** `build.rs` reruns on
`MCF_BUILD_COMMIT`; `build_identity.rs` reads it through `option_env!` and — per
A7 — refuses to invent one when absent; `licence.rs` explains the absence to
the operator. Careful work. And the variable is set in exactly one place in the
repository:

```
scripts/check-reproducible-build.sh:79:  export MCF_BUILD_COMMIT="$revision"
```

The binary an operator builds with `cargo build --release` never has it, so the
honest field honestly reports `Unknown`, every time, for ever. **A field that is
always unknown is not a mechanism.**

**Why this is worse than the three defects it hides.** F90 and F91 were
*fixable* because their extent could be reasoned about — from git, from memory,
and from a physically impossible reading of 44 cores on a 32-thread machine.
None of that reasoning came from the record. The next defect will be subtler and
there will be nothing to reason from. This is the defect that makes the others
unrecoverable, and a repository that found three in one day will find more.

**The fix, going forward: the binary's own digest.** MCF hashes its own
executable once and records it with every measurement. No build cooperation, no
git, nothing an environment can forget to supply, and it works for a binary
shipped in a tarball with no `.git`. Two builds that measure differently have
different digests by construction, which is the only property needed to
partition a record correctly. This machine's records now carry
`"instrument": "12ce9405…"`. It says *that* the instrument differs rather than
*what* changed — and what changed is the erratum's job.

**The fix, backwards: an erratum keyed on time.** The 52 already written cannot
be attributed by digest. But `recorded_at` is on all 3391 entries, precise and
trustworthy, and the correction times are known. So `mcf_core::errata` carries
each defect — the instrument, what was wrong, **what it did to the readings**,
and the moment it was corrected — and every surface that renders a measurement
renders the errata that apply to it:

```
comparison_2026-08-28T17-20-03Z…  compared … 
  ⚠ ERRATUM mcf_core::hardware::contention: the rate divided accumulated
    processor ticks by the interval the sampler intended to wait …
  ⚠ ERRATUM mcf_core::hardware::thermal: no processor temperature was read …
  ⚠ ERRATUM mcf_bench::enough: the effect size was a point estimate …
```

The entry recorded at 22:59 UTC gets **only** the effect-size erratum, because
the other two were already fixed by then. That is per-entry attribution from
timestamps alone.

**Nothing is rewritten** (A1). The measurements stay exactly as taken — they are
what the instrument said. A record that edits its own history to look better is
not a record. And each erratum states its *effect*, not merely that something
was wrong: a reader told only *distrust this* cannot decide what to do, while a
reader told *high by about a fifth under load* can.

**A defect inside the fix, caught by its own test.** The first version of the
errata list carried nanosecond constants **two days** from the dates written
beside them — computed by hand, entirely plausible on sight, and silently
wrong, so no erratum applied to any entry and the whole mechanism did nothing
while appearing to work. Two spellings of one fact drift. The test now
cross-checks the nanoseconds against the readable moment through `Timestamp`,
which is A19: a reported quantity checked against an independently known value.

That is the sixth time in this session that a thing which looked correct was
wrong, and the second where the failure mode was *doing nothing while appearing
to work*.

## 94 · F94 — A19 was applied to everything MCF computes and nothing MCF measures (A19, §6.16, F90, F91, F92, F93)

**Six defects in one session, and 1,321 test functions found none of them.**

| | defect | found by |
|---|---|---|
| F80 | register headline wrong by five items | adding a row and noticing the total |
| F84 | branch read from the wrong axis | running a new rule against real call sites |
| F90 | contention +22 %, above the physical ceiling | an operator asking about headroom |
| F91 | no processor temperature, ever | an operator saying the data was accessible |
| F92 | effect size with no measure of itself | reading the output as a stranger |
| F93 | instrument unidentifiable in every record | asking why F90's extent was knowable |

Not because the tests are weak. Because every one of these was code doing
exactly what it was written to do, and the writing was wrong. A test written
from the same understanding as the code cannot see past it.

**What unites them: an independent source existed and was never consulted.**
`/proc/stat` for contention. A second sensor directory for temperature. An
independent computation of a textbook interval for the effect size. The record
itself for the instrument's identity. None was hard to find — `/proc/stat`
exposed F90 in about thirty seconds once somebody looked.

**MCF has a rule for exactly this, and it names this failure.**

> **A19 — Anything reported is tested against an independently known value.**
> **Violation looks like:** *a statistic whose only validation is that it looks
> about right.*

**Where it was applied.** Fifty-eight files. SHA-256 against published vectors,
the tokenizer against reference output, dequantization against `llama.cpp`,
time arithmetic against known dates, JSON, HTTP framing, the control protocol,
the probes. Genuinely thorough.

**Where it was not.** Zero citations in `hardware/contention.rs`,
`hardware/thermal.rs`, `bench/enough.rs` or `bench/project.rs` — every module
that measures the machine or reduces measurements to a claim.

And the sharpest form of it: MCF ships `mcf cross-check`, which compares its own
inference engine against an independently provisioned reference across a
hundred and twenty positions, chosen because F40 found two engines parting at
step four. That discipline was applied to the engine, and to nothing that
measures.

**Why the asymmetry is understandable and still wrong.** A digest has a
published test vector; a machine's contention does not. So the parts with
obvious ground truth got A19 and the parts without it got careful reasoning
instead. But an independent source existed in every case — what was missing was
the requirement to look for one.

**What was built.** `scripts/ci.sh --with-instruments`, scheduled rather than
gating because it needs the real machine and warms it for eight seconds:

- **contention against the kernel's own accounting** — a different file, a
  different accounting path, and a quantity that cannot exceed the core count;
- **a processor sensor against physics** — every core is loaded for eight
  seconds and the die must get hotter. A board zone read as a die, a stale
  value, or the wrong sensor entirely all fail this, which is precisely F91's
  ACPI zone at 16.8 °C standing in for a processor at 70 °C;
- **the interval's coverage against brute-force enumeration** — every one of
  the 2ⁿ sign patterns counted directly, the definition with no algebra in it,
  against the closed form the implementation uses;
- **occupancy against the vendor's own tool** where one is installed.

**It found something on its first run.** The enumeration and the closed form
disagreed by **one part in a million**: at eight pairs the true coverage is
99.21875 %, and MCF stated 99.2188 % where enumeration gives 99.2187 %.
Computing `MILLION - missed` truncates the part being subtracted, which rounds
the *coverage* up. The magnitude is trivial and the direction is not — a
coverage MCF overstates is a guarantee it cannot keep. It now counts the
patterns that fall inside and truncates those, so the claim is never larger
than the truth.

That is a defect no reasoning would have found and no test written from the
same understanding would have caught. It took a second implementation that
shared no arithmetic with the first.

**A disagreement is a finding, not a crash.** Each check reports both values and
how far apart they are, because *the instrument is wrong* is not actionable and
*it reads 35.2 where the kernel reads 28.9* is.

**The general lesson, stated for the next instrument.** The failure mode of a
test is not a wrong answer — it is agreement with the code it was written
beside. Six times in one session, the thing that broke the agreement was an
outside view: a kernel counter, a physical prediction, a second implementation,
or an operator asking a question the code had not anticipated.

## 95 · F95 — The band is at a third of the machine, and half a machine free is not enough (DEC-007, B-217, B-084, §3.8, F90, F92)

**The measurement DEC-007 asked for, finally possible.** DEC-007 settled that
quiet is relative — *a machine steady throughout a run is measurable wherever
its baseline sits* — and left open the band, insisting the number be measured
rather than chosen. It could not be measured until today, because there was
neither a trustworthy axis to measure against (the contention instrument
over-read by a fifth under load, F90) nor a quantity to measure (the effect
size was a point estimate with no measure of itself, F92). Both are fixed.

**What is read out is width, not size.** A machine that cannot be measured is
not one whose answers get *bigger* — it is one whose answers get *wider*. So
the observable is the width of the effect-size interval, and every run is cut
to the same first six pairs before it is computed, because the stopping
condition takes as many pairs as it needs and comparing widths at different
counts would be comparing the stopping condition with itself.

**The sweep.** The same paired comparison — two SmolLM2 quantizations, 32
tokens, cold — at rising fractions of this 32-thread machine, three times each:

| load asked | competing, measured | interval widths (points) | against baseline |
|---|---|---|---|
| 0 % | 4–7 % | 6.7, 10.5, 12.7 | — |
| 25 % | 28–30 % | 8.7, 11.4, 15.0 | **indistinguishable** |
| 50 % | 51–55 % | 65.3, 82.6, 309.0 | **6× to 30×** |
| 75 % | 76 % | 123.3 | 12× |

**The band is between 30 % and 51 % of machine capacity.** At 30 % the widths
(8.7–15.0) sit inside the baseline's own run-to-run range (6.7–12.7) — the
criterion the operator set, and one that chooses no number: the machine's own
quiet variability is the threshold. At 51 % the smallest width is more than
five times the baseline's largest, with no overlap at all.

**Half a machine free is not enough**, which is the result worth carrying. The
operator's question several rounds ago was whether F74's noisy benchmarks were
simply a machine with headroom to spare. They were not — those ran at 85–113 %
of capacity — but this sweep answers the general form of the question, and the
answer is that headroom is the wrong frame. With **sixteen of thirty-two
threads idle**, the interval is already six to thirty times wider. Degradation
does not wait for saturation.

**And the corrected instrument validated itself in the field.** The load was
generated in known amounts and MCF's own contention reading was taken during
each run: eight spinner threads read as 8.98–9.62 cores, sixteen as
16.33–17.65, twenty-four as 24.27. F90's fix is not merely internally
consistent; it agrees with a quantity that was set rather than observed.

**What it costs to run a benchmark on a busy machine**, incidentally measured:
a single 32-token cold generation takes about 1.5 seconds idle and about 60
seconds at 75 % load — **forty times slower**. A comparison that finishes in
half a minute on a quiet machine took twelve minutes.

**Two levels were planned and not taken.** The sweep was designed for six
levels to 125 % of capacity and was stopped after four. The reason is itself a
measurement: at 100 % of capacity a single comparison did not finish inside the
fifteen-minute cap the harness allows it, having taken about thirty seconds on
an idle machine. Estimating the sweep at forty-five to ninety minutes was
wrong by more than a factor of two, for the same reason the finding is about —
work on a saturated machine does not slow down proportionally. The four levels
taken bracket the band; the two not taken would have confirmed a trend already
unambiguous across them, at the cost of another hour of somebody's machine.

**What this does not settle.** The bracket is 30–51 % and no finer: nothing was
measured between them, so the departure could be anywhere in that range. One
model pair, one token budget, one machine, three repeats a level. The shape is
clear and the edge is not, and locating it would take a second sweep across
31–50 % rather than a wider one.

## 96 · F96 — The pre-flight marks rather than refuses, and the band it uses says whose machine measured it (B-217, DEC-007, F95, A21, A20, A4)

**B-217, asked for as a refusal and built as a marking.** The item wanted a
laboratory that *refuses to begin on a contended machine rather than producing
an invalid result*. [F71](#71--f71--the-machine-either-side-of-a-run-is-a-condition-not-a-gate-b-217-dec-007-34-38-a6-a7)
built the measurement and left the refusal, because the threshold would have
been a figure MCF chose. [F95](#95--f95--the-band-is-at-a-third-of-the-machine-and-half-a-machine-free-is-not-enough-dec-007-b-217-b-084-38-f90-f92)
measured the threshold. This is what was built on top of it, and it is not a
refusal:

```
NOT FIT TO CONTRIBUTE — 84.3% of this machine was already busy (27.00 of 32
core(s)) — OUTSIDE the band of 30.0%, so this is a real measurement that is
not fit to contribute (B-217, DEC-007). The band is DECLARED from one
32-thread machine, one model pair, 32 tokens (F95); `prototypes/contention-band`
measures it here (A21, A20)
```

**Why marking and not refusing** (the operator, 2026-08-28). A run outside the
band still happened, and A4 keeps what it produced: the pairs are real, the
verdict stands, the record is complete. What it loses is the right to travel.
Refusing to start would deny a result to anyone whose machine is simply busy,
and this project's answer to that has been the same in three findings now —
measure the condition, attach it, and let the reader judge.

**A fraction of capacity, not a count of cores.** Sixteen busy threads is half
of a thirty-two-thread machine and a sixteenth of a large one, and F95 measured
the departure at a *fraction*. So `Headroom` divides by what
`available_parallelism` reports, and the same absolute load is inside the band
on one machine and outside it on another — which is DEC-007's *quiet is
relative* expressed in the units that predict the harm.

**The band is somebody else's measurement, and says so** (A21, B34, A20).
Thirty percent came from one machine, one model pair, one token budget. That
makes it **declared** here in exactly the sense F42's declared context length is
declared: far better than a figure chosen out of the air, and not the same as a
local measurement. The rendering carries the provenance and names
`prototypes/contention-band` as what replaces it — an estimate replaced by a
measurement, never promoted into one.

**Every reason, not the first.** `Finding::not_fit_to_contribute` returns a
list. A run can be outside the band *and* fail to establish its size (F92), and
a reader told only one of those will fix that one and be surprised again (A1).

**Distinct from `Withheld`, which was the temptation.** `Withheld::Confounded`
and `Withheld::MixedReuse` suppress the delta — those comparisons have no delta
to give. A busy machine is not that: the delta exists, it is real, and it is
worth less. Reusing `Withheld` would have thrown away a measurement in order to
mark it, which is the opposite of what the operator chose.

**And a machine nobody read is not a machine that was free** (A7). `Headroom`
is `Option` on the comparison, absent for anything built from a record that
never carried it, and an absent reading marks nothing and claims nothing.

## 97 · F97 — Five modules measure something and are checked against nothing, and now they say so (B-390, A19, A7, §6.16)

**The half of B-390 that was owed.** [F94](#94--f94--a19-was-applied-to-everything-mcf-computes-and-nothing-mcf-measures-a19-616-f90-f91-f92-f93)
built the tier that compares each instrument against an independent source. The
operator chose *tier **plus** a check*, and without the check the next
instrument arrives the way the last four did.

**Every module in the measuring crates now declares one of three things**, in
its own documentation where the next person to edit it will read it:

| marker | meaning |
|---|---|
| `**Cross-checked by test:**` | names a test in the instrument tier, which the check looks up |
| `**Cross-checked by:**` | names something that is not a test — two independent routes compared inside the reading itself |
| `**Cross-check owed (B-390):**` | names what would serve, and does not have it |

**The third is the honest one.** Five modules measure something and are
compared against nothing: `processor` reads core counts and the governor from
one place; `scheduling` measures delay against MCF's own clock alone; `space`
and `storage` read one call each; and `project` is graded by leaving each point
out and projecting it from the others (F68) — which is the instrument checking
itself, not an independent source. Writing *cross-checked* on any of those
would have been worse than the gap.

**So the count is a ratchet.** Five may fall and may not rise. A new instrument
cannot quietly join the list: it has to be cross-checked, or argued to measure
nothing, and both of those are visible in review. When the debt falls the check
insists the number be lowered to match, so it cannot creep back up.

**A marker in the module, not a list in the check.** A list here goes stale the
week it is written — this repository has said so about tables of driver names
(F79) — and worse, it is read by nobody editing the instrument. The marker is
where the work happens.

**And the check caught its own imprecision on the first run.** It tried to tell
a test name from any other backticked identifier and could not: it read
`routes_disagree_about`, a struct field, as a test that did not exist. The fix
was to make the claim *stated* rather than inferred — a separate marker for the
form that names a test. That is the same lesson as `LocallyMeasured::new` not
being a `From` (B-167): a claim about where something came from should be
written rather than guessed at.

**Both negative controls were exercised.** A module whose marker was removed
fails by name; a sixth module claiming the debt fails with *up from 5*.

## 98 · F98 — The unpaired interval needed different mathematics, and the recurrence was checked against enumeration (B-388, B54, B53, §3.27, A19)

**The gap [F92](#92--f92--the-headline-number-had-no-measure-of-itself-and-the-sentence-beside-it-claimed-otherwise-b46-b54-a6-616-327)
left open.** When the effect size became a range, the range was an order
statistic of *paired* differences with coverage from a binomial tail. Arms
assembled from separate sessions have no pairs, so `Verdict::Apart` reported a
point and said so — because inventing a range from the two arms' own ranges
would have been exactly the confident wrong number the interval exists to
prevent.

**What it needed.** Without pairs the estimator is the median of **every**
pairwise comparison — `n·m` of them — and the coverage comes from the rank-sum
distribution rather than from a coin. That distribution is exactly computable
by a recurrence: the next value comes either from one arm, adding `m` to the
statistic, or from the other, adding nothing.

```
f(n, m, u) = f(n-1, m, u-m) + f(n, m-1, u)
```

Whole numbers throughout, no resampling, no distributional assumption — the
same standard the paired interval is held to, reached by different means.

**On the quantity MCF already reports.** Each pairwise value is the signed
difference in parts per million of the smaller of the two. That is a *monotone*
function of the ratio between them — increasing in `y/x` on both sides of the
crossover, which is what makes an order statistic of these values an order
statistic of the ratio and the inversion valid.

**The recurrence was checked against enumeration before it was used.** For
every `n` and `m` up to six, a second implementation generates every
interleaving of the two arms as a bit pattern, tallies the statistic directly,
and compares. The two share no arithmetic. They agree, the counts sum to
`C(n+m, n)`, and the distribution is symmetric — which is [F94](#94--f94--a19-was-applied-to-everything-mcf-computes-and-nothing-mcf-measures-a19-616-f90-f91-f92-f93)'s
discipline applied at the moment of writing rather than after a defect.

**A cap, and which way it errs.** The table is `n·m + 1` counts built in `n·m`
steps, so its cost grows as the square of the product. Forty a side is sixteen
hundred pairwise values — far more evidence than any run here has produced —
and beyond it both arms are scaled down in proportion. Scaling down **widens**
the interval: an arm treated as smaller than it is claims less than it could,
which is the direction an approximation is allowed to err in.

**`Apart` stays its own verdict.** It now carries the same three fields the
paired row does, so a reader comparing the two sees both stated the same way —
and it remains a distinct kind in the record, because what these arms lack is
the pairing that would have held the afternoon still, not an interval. B53's
*weaker claim* is about the construction, and that has not changed.

**And the debt is settled.** Of the five modules that measure something and are
compared against nothing, `enough` was never one — it was cross-checked from
the day the interval landed. This adds a second independent check to the same
module rather than a sixth entry to the list.

## 99 · F99 — Threads pay at every shape a model performs, cost nothing in noise, and past a product's optimum more of them is slower (B-366, D38, F52, §3.12, A6, A19)

**What D38 asked for and what was owed.** Work split across processors, with
**bit-identical output whatever the thread count** — and
[F52](#52--f52--the-engine-benchmarks-will-use-is-three-times-noisier-than-the-one-they-will-not-and-two-guesses-about-why-were-both-wrong-dec-007-b-366-b-376-34-a19-f51)
standing beside it as the reason not to predict any of it: thread count moved a
benchmark's noise by a factor of five, in the direction opposite to the one
reasoned out, and the mechanism was never established. So none of the numbers
below was estimated first.

**Every figure was taken with the machine held to itself** (`heavy`, load
average 0.47 to 1.03 across the run), on one thirty-two-processor machine, with
the stand-in reading `SmolLM2-135M-Instruct-Q8_0`. They are facts about MCF's
own engine on one machine, not measurements MCF publishes: B65 forbids a speed
from a stand-in, and this is the same class of evidence as F8, F12 and F52.

### The answer never moved

Every cell of every table below was compared against the one-thread answer.
Eight product shapes × seven thread counts × nine repeats, and a whole
generation at seven counts × five repeats: **identical, always**. That is
B-366's claim asked of a real model rather than of a fixture.

### Every product a model performs is faster partitioned

`211 products a token` on this model — 210 of them under a million elements and
one output projection of 28 million — so the worry going in was that starting
workers 211 times would cost more than it bought. It does not:

| rows × columns | 1 thread | best | speedup |
|---|---|---|---|
| 576 × 576 (an attention projection) | 492 µs ±1% | 129 µs ±2% at 16 | 3.8× |
| 1 536 × 576 (a feed-forward) | 1 338 µs ±1% | 196 µs ±10% at 16 | 6.8× |
| 2 048 × 2 048 | 6 531 µs ±2% | 550 µs ±3% at 16 | 11.9× |
| 5 632 × 2 048 | 18 012 µs ±1% | 1 166 µs ±6% at 32 | 15.4× |
| 4 096 × 4 096 | 25 462 µs ±1% | 1 617 µs ±4% at 32 | 15.7× |
| 11 008 × 4 096 | 68 446 µs ±1% | 5 015 µs ±5% at 24 | 13.6× |
| 32 000 × 2 048 (an output projection) | 101 609 µs ±1% | 6 931 µs ±4% at 24 | 14.7× |
| 49 152 × 576 | 43 303 µs ±1% | 3 169 µs ±3% at 24 | 13.7× |

Even the smallest product a 135-million-parameter model performs is nearly four
times faster partitioned. End to end the model goes from **347 ms a token to
66** — 5.2× — which is the difference D38 was after between an engine that runs
a large model and one that theoretically would.

### But past a product's optimum, more workers is *slower*

This is the part no reasoning produced. Measured before any rule was applied:

| rows × columns | 8 threads | 16 | 24 | 32 |
|---|---|---|---|---|
| 576 × 576 | **130 µs** | 197 | 221 | 296 |
| 1 536 × 576 | 239 | **196 µs** | 262 | 322 |
| 2 048 × 2 048 | 940 | **549 µs** | 598 | 605 |
| 5 632 × 2 048 | 2 465 | 1 388 | 1 353 | **1 211 µs** |

A 576 × 576 product is **2.3× slower on thirty-two workers than on eight**.
Handing every product every processor the machine has would therefore have been
slower than handing it some of them — and end to end that is exactly what
happened: without a rule, thirty-two threads ran the model at 912 ms against
557 ms at eight, **1.64× worse than the best count**.

### So the engine spends only what a product earns

One number, and it comes from the two shapes that bound it: a 331-thousand-element
product was fastest at eight workers (41 thousand each) and an 885-thousand one at
sixteen (55 thousand each). `WORTH_A_WORKER` is **50 000 elements**, and a product
gets `elements / 50 000` workers or the caller's thread count, whichever is fewer.

**What the rule buys is not speed — it is that asking for everything stops
costing anything:**

| threads asked for | without the rule | with it |
|---|---|---|
| 8 | 557 ms (4.77×) | 603 ms (4.60×) |
| 16 | 567 ms (4.69×) | 552 ms (5.03×) |
| 24 | 737 ms (3.61×) | **530 ms (5.23×)** |
| 32 | 912 ms (2.91×) | 535 ms (5.19×) |

The peak barely moved (4.77× to 5.23×). The *penalty for asking for too much*
went away, which is what makes *what this machine reports* a safe default rather
than a number somebody has to tune per model. **A prediction made before
measuring was wrong by a factor of two**: the rule was expected to roughly halve
the per-token cost, by giving the small products fewer workers and the output
projection all of them. It did not, because the 210 small products dominate and
the rule gives them *fewer* workers, not more.

**The mechanism is not established and nothing here claims one.** Whether this
is thread-start cost, memory bandwidth, or the cache behaviour of a matrix that
stops fitting, was not measured. F52 is the standing evidence that guessing at
it produces backwards answers.

### Threads did not make MCF's engine noisier

F52's caution was explicit: *giving MCF's own engine threads will change its
noise characteristics as well as its speed, and the change must be measured
rather than predicted.* Measured, the middle half of five repeats:

| threads | 1 | 2 | 4 | 8 | 16 | 24 | 32 |
|---|---|---|---|---|---|---|---|
| middle half | 0.5% | 1.9% | 1.9% | 0.8% | 0.5% | 0.1% | 0.7% |

**No worse at thirty-two threads than at one**, and tighter than the 3.6% F52
measured for this engine single-threaded through the daemon. The prediction F52
declined to make would have been wrong in the pessimistic direction.

### On a busy machine, threads cost rather than pay

The first pilot ran on this machine at load average **34 of 32 processors**, and
every thread count was *worse* than one, monotonically: 113 ms a token at one
thread and 539 at thirty-two, 5.3× slower. The same command on the quiet machine
is 5.2× **faster** at thirty-two. So the benefit of a thread count is a condition
of the machine's state and not a property of the engine, which is F52's own
lesson arriving from the other direction — and it is why the number MCF renders
beside an answer is the count it used rather than a claim about what that count
bought.

### The rewrite did not cost the one-thread path

`matmul_vec` became a delegation to the partitioned form so that both paths sum
a row with the same function. That is a change to the code the *default* path
runs, so it was measured against the loop it replaced, kept in the prototype for
the purpose. Across all eight shapes the rewritten path was faster, by 20% on the
smallest and by 0.5–2% on the rest — consistent in direction, and inside the
repeats' own spread on every shape but the smallest.

### What this did not establish

Anything about another machine: one processor, one memory system, one thread
count. Whether the 50 000 holds on a machine with four processors or a hundred
and twenty-eight — the rule errs toward fewer workers, which costs speed there
and can never cost correctness. Anything about a model larger than 135 million
parameters, which is [B-384](backlog.md)'s question and the reason it was
sequenced after this one. And energy, thermal state and memory, none of which
was read.

## 100 · F100 — How large a model MCF's own engine can usefully read, and what actually stops it (B-384, PR12, D40, B-366, F99, A20, A7)

**The fact nobody had.** PR12 was accepted on the condition that B-366 go first
*because it moves the number*, and then that somebody measure how large a model
MCF's own engine can usefully read — which decides whether twenty-seven billion
was ever the target. Nothing between 0.6 billion and 27 billion had been tried.
Four models were acquired to fill it: Qwen3 at 0.6B, 1.7B, 4B and 8B, one
family across the range, meeting the reference model already held.

**Every figure was taken with the machine held to itself** (`heavy`, load
average 1.4 to 3.3), on one thirty-two-processor machine, four tokens after a
five-token prompt, greedy, seed 0. Facts about MCF's own engine on one machine,
not measurements MCF publishes (B65).

### The ladder

| model | elements | multiplied | dequantized | per token | 120 positions |
|---|---|---|---|---|---|
| stories15M Q8_0 | 24M | 15M | 97 MB | 15 ms | 1.8 s |
| SmolLM2-135M Q8_0 | 134M | 106M | 538 MB | 84 ms | 10.0 s |
| Qwen3-0.6B Q8_0 | 596M | 440M | 2 384 MB | 221 ms | 26.5 s |
| Qwen3-1.7B Q8_0 | 1 720M | 1 409M | 6 882 MB | 479 ms | 57.4 s |
| Qwen3-4B Q4_K_M | 4 022M | 3 633M | 16 089 MB | 977 ms | 117.2 s |
| Qwen3-8B Q4_K_M | 8 190M | 7 568M | 32 762 MB | **1 744 ms** | **209.2 s** |
| Qwen3.8-27B Q4_K_M | 27 320M | — | 109 282 MB | — | refused |

**The size that stops the reference model is not its size.** 27,320M elements
and 109,282 MB dequantized is PR12's figure exactly — and MCF never reaches the
point of caring, because the file declares architecture `qwen35`, which is not
one of the three MCF has been taught. A ceiling argument was built on a number
that is correct and is not the blocker. The register item for the architecture
is B-365.

### What "usefully" means here, and why it is not a preference

Tied to the purpose that justifies the engine existing: A19 and D31 put it here
to be **checked against an independent implementation**, so the number that
decides the ceiling is what that check costs. F49's cross-check is a hundred and
twenty positions, which is the last column above — **3.5 minutes on an 8B
model.**

**PR12's arithmetic is superseded by roughly an order of magnitude.** It
measured Qwen3-0.6B at 0.9 s a forward pass and projected the reference model's
cross-check at **eighty minutes**. That model now costs 221 ms, and the same
projection to 27 billion elements is **about twelve minutes**. PR12's
recommendation — *do not lift the ceiling by dequantizing per use, it spends
time the engine does not have* — rested on a margin that has changed by that
factor. This does not overturn it: what per-use dequantization actually costs
per token has never been measured, and that measurement is what the question now
needs. It does mean the recommendation should be re-asked rather than inherited.

### The curve was the threads, and two explanations were wrong first

Cost per token is **sub-linear in a model's elements** — 0.67 µs per thousand
elements at 24M against 0.22 at 8 190M — which reads as an engine that gets more
efficient at scale.

*The first explanation was the denominator.* A token's embedding is one row read
out of the embedding table, not a product against all of it, and for a small
model that table is most of the file: nine of stories15M's twenty-four million
elements are vocabulary. So the ladder counts what a forward pass actually
multiplies. **It was a real effect and not the answer** — the rate still fell
fourfold across the range.

*The second explanation was [F99](#99--f99--threads-pay-at-every-shape-a-model-performs-cost-nothing-in-noise-and-past-a-products-optimum-more-of-them-is-slower-b-366-d38-f52-312-a6-a19)'s
partition*: a small model's products earn few workers and a large model's earn
all of them. That is a claim with two outcomes, so it was measured rather than
argued — the same ladder pinned to one thread:

| multiplied elements | at one thread | at thirty-two | speedup |
|---|---|---|---|
| 15M | 3 933 µs/M | 1 000 µs/M | 3.9× |
| 106M | 4 405 | 792 | 5.6× |
| 440M | 4 604 | 502 | 9.2× |
| 1 409M | 4 250 | 339 | 12.5× |
| 3 633M | 3 946 | 268 | 14.7× |
| 7 568M | 3 437 | 230 | **14.9×** |

**At one thread the rate is flat** — 3 400 to 4 600 µs per million multiplied
elements, no trend with size. **At thirty-two it falls monotonically by 4.3×.**
The engine's arithmetic is linear in the work it does; the *partition* is what
scales with size, because a bigger product earns more workers. The curve was
never a property of the arithmetic.

### So: the ceiling, stated

**Time is no longer what binds.** Memory is: the engine dequantizes to `f32` on
load (D38's legibility trade), so the ceiling is available memory divided by
four bytes — **about 18 billion elements** on this 91 GiB machine, where 72 GiB
was free. Projected at that ceiling from the measured rate: roughly **4 s a
token and an 8-minute cross-check** — an estimate, labelled, and never promoted
(A20).

**Measured, the largest model that runs here is 8 billion elements**: 9.3 s to
load, 1.7 s a token, 3.5 minutes for the check that justifies the engine.

### What this changes about the validation half

D40 split the reference role: the reference model is the subject of provenance,
frontier and residency work, and what the *engine* is developed against is a
conformance corpus of **the smallest trained model of each family**. That rule
was correct when it was written, and the one-thread column is what it was
correct against — an 8B model then cost **52 minutes** for one cross-check, and
a corpus made of such models could not be run.

It now costs 3.5 minutes. So *smallest of each family* is no longer forced by
cost, and continuing it would be choosing by convenience — which is exactly what
B-384 exists to prevent. The corpus gains one rung at the top: an 8B model, the
largest that runs here, because every entry until now has been under 600
million and a defect that needs thirty-six blocks or a wide grouped-query ratio
to appear has never had a file that could show it.

### What this did not establish

One machine, one memory system, one processor. Whether the flat one-thread rate
holds on hardware with a different cache hierarchy. What per-use dequantization
costs, which is the measurement PR12's question now needs and which nobody has
taken. Anything about a model between 8 and 18 billion elements — the projection
to the memory ceiling is extrapolation from six points, and the honest form of it
is the word *estimate*.

## 101 · F101 — A model called the tool perfectly and the probe recorded *no call*, five times out of five (B-053, D42, A1, A2, A21, §3.18)

**The first run against a real model found a defect in the reader, which is
what a real model is for.** `mcf probe` on Qwen3-0.6B, through the provisioned
llama.cpp at a pinned commit, reported:

| offering | well formed | malformed | no call |
|---|---|---|---|
| the file's own markers, `<tool_call>…</tool_call>` | 0 | 0 | **5** |
| a plain description, a bare object expected | **5** | 0 | 0 |

*No call, five times out of five* — from a model that calls perfectly under the
other offering. The reader looked only **between** the markers when an offering
named them, so a model that emitted a correct call *without* the wrapper was
recorded as having done nothing at all.

**Two facts had been folded into one, and the wrong one survived.** *It ignored
the tool* and *it called, but not in the form asked for* are different things to
know about a model, and A1 forbids discarding the second. It is also A2's silent
failure with a friendly name: the probe reported a clean, plausible, wrong
answer, and nothing in it looked broken.

**What the model actually did, once the reader could say so.** Every one of the
five trials, byte for byte:

```
{"name": "get_weather", "arguments": {"city": "Paris"}}
```

A well-formed call to the offered tool, with the right argument, emitted with no
markers around it — despite MCF's instruction naming the markers and the model's
own vocabulary carrying them as tokens. The corrected reading is `malformed`
with that text quoted, because the form asked for was not produced and the model
plainly did call.

**And that is a fact about tool calling worth keeping.** The markers a file
declares are what its *harness* wraps a call in, not what the model emits.
`<tool_call>` is in the vocabulary because the template writes it; the model was
trained to produce the object and to let the framing be somebody else's job. So
a vocabulary carrying call markers is a declaration about the **format a harness
must speak**, and never a prediction of what the model will emit — which is
A21's separation arriving in a place nobody had looked for it.

**The design decision this vindicates.** [B-053](backlog.md)'s probe tries
*several* offerings rather than the one its file suggests, because MCF does not
execute chat templates (D46) and therefore chooses how a tool is described — and
a single chosen framing deciding the answer would make the probe a measurement
of the framing. Had it tried only the likeliest candidate, the model's own
markers, it would have reported **no tool support** for a model that calls
correctly every time.

**A smaller thing from the same run.** The declared markers printed as
`<tool_call>, </tool_call>, <tool_call>, </tool_call>, <tool_response>, …` —
`markers_in` reports every occurrence and a template that writes a marker five
times is not five declarations. Deduplicated.

**What this did not establish.** Whether another family emits its markers when
asked to; one model, one engine, one phrasing of the instruction. Whether a
model that *is* given a template-rendered tool section behaves differently,
which MCF cannot ask without executing templates. And nothing about whether the
call is *useful* — the arguments are deliberately not read for sense, because
that is a judgement and a probe makes none (D42).

## 102 · F102 — The conformance corpus answered differently depending on whether a daemon was running (B-370, B-053, F46, F27, D40, §3.12, A19)

**Found by accident, which is the only reason it was found.** Twelve of the
corpus's sixteen entries were not on this machine, so the check had been
exercising a quarter of what it claims. Re-acquiring them
([B-384](backlog.md)'s work) made one entry fail — and the failure was not in
the model, the engine, or that day's changes.

**`check-corpus.sh` ran `mcf run` without naming an engine.** That means
*whatever daemon happens to be listening*. Measured on one model, one machine,
one afternoon:

| how it was run | what came out |
|---|---|
| MCF's own engine, no daemon in the path | `the city of Paris. The city of Paris is the city` |
| through the daemon, **engine unstated** — what the check did | `a city in the same country. The city is in the` |
| through the daemon, stand-in named | `the city of Paris…` |
| through the daemon, provisioned named | `a city in the same country…` |

The check passed all morning while the daemon was down and failed the moment
one was up. Nothing in the check could see the difference, and the entry that
flipped had been on the disk for twenty minutes.

**This is the third occurrence, and it landed exactly where the guard against
it does not reach.** [F46](#46--f46--mcfs-own-default-was-cutting-every-answer-off-and-a-gate-test-that-depends-on-whether-a-daemon-is-running-b-056-b-059-d42-d43-38-312-b49-f38)
found the first, [F47](#47--f47--the-suite-was-reporting-on-the-machine-it-found-b-378-b-003-b16-312-f46)
fixed it by making the socket an *input* and built
`checks/tests/no_test_reads_ambient_state.rs` so it could not come back — and
F47 named its own limitation in writing: *"What was not established. Whether
other tiers have the same dependency."*

**This is that question answered, and the answer is yes.** The guard watches
**Rust test files** for calls to ambient-reading functions. `check-corpus.sh` is
a shell tier that runs the *built binary*, so there is no call for it to see:
the ambient read happens inside `mcf run`, legitimately, because a person typing
`mcf run` should get whatever engine is serving. The defect is not in the binary
at all — it is that a **check** invoked it the way a person would, and a check
is not a person. §3.12 does not permit a suite whose answer depends on the state
it found, and the shell tiers were never brought under that rule.

It also survived because most of the corpus was absent — **a check that cannot
run cannot be caught being wrong.**

**The fix is to state the engine rather than inherit it.** The stand-in, because
D40 makes this corpus the thing *MCF's own engine* is developed against, and the
header now prints which engine every entry went through. `--engine stand-in`
works with or without a daemon, so the check is hermetic in the sense B19 wants.

**The second finding, underneath the first: the two engines genuinely disagree
on this file, and the oracle was right not to care.** `Qwen3-0.6B-Q2_K` is the
most aggressively quantized model in the corpus, and
[F29](#29--f29--the-sixth-family-answers-a-different-question-b-371-dec-055-b-365-a19-33)
and F33 already record that MCF multiplies floats where the reference multiplies
in quantized arithmetic, with the widest gap observed **on a Q2_K file for
exactly that reason**. The oracle ran on this same model in the same session and
reported agreement on all 142 comparisons — because it compares *distributions
against a stated margin* (F27's 0.40) rather than an exact string from a greedy
decode.

So the corpus's `says Paris` test is the more brittle instrument of the two: on
a knife-edge token, two engines that agree within the oracle's tolerance produce
different words. That is not an argument for loosening it — an exact string is
what makes a corpus entry mechanically checkable — but it is the reason the
entry has to say which engine it is an expectation *about*, which it now does.

**And the other shell tiers have not been audited.** F47's survey was "a survey
of three, not of the suite", and this adds one more to it. `check-oracle.sh`,
`check-seed-set.sh` and the rest each invoke the binary, and whether any of them
inherits an engine, a store or a socket rather than stating it is unexamined.
That is the open question this finding leaves, in the same words F47 left it.

**What this did not establish.** Whether other corpus entries are equally close
to their knife-edge, which would need the same comparison run per entry.
Whether the provisioned engine's answer is worse — neither continuation is
wrong, and nothing here judges a model's output for sense.

## 103 · F103 — The oracle could compare the reference with itself, and a section that compared nothing read like one that passed (B-368, B-370, F102, F47, A19, A4, §3.12)

**Found by pulling the thread [F102](#102--f102--the-conformance-corpus-answered-differently-depending-on-whether-a-daemon-was-running-b-370-b-053-f46-f27-d40-312-a19)
left.** That finding fixed one tier and wrote down what it had not looked at:
whether the other shell tiers inherit an engine the same way. They do, and one
of them is the oracle.

**What the audit found.** Four `mcf run` invocations across the scheduled tiers
named no engine, so each used whatever daemon was listening: `check-corpus.sh`
(fixed in F102), `check-online.sh` — whose own heading reads *"and running it,
with the engine MCF wrote"* — and `check-oracle.sh` in three places. The two
`mcf embed` calls are **not** affected, because `mcf embed` loads in-process and
never consults a daemon; that was checked rather than assumed.

**The serious one.** `check-oracle.sh` builds its comparison as

```
mine=$("$mcf" run "$model" …)          # meant to be MCF's own engine
theirs=$("$completion_reference" …)     # the reference implementation
```

With a daemon serving the provisioned llama.cpp, `mine` *is* llama.cpp. The
oracle would then compare the reference implementation **with itself** and
report agreement — in the check that is A19's mechanical form, and on which
every other claim in this repository leans. A check that passes because it has
stopped testing is worse than one that fails.

**Measured, not inferred.** One model, one prompt, one seed, with a daemon
serving the provisioned engine — the three answers the oracle's two variables
can take:

| | what came out |
|---|---|
| `theirs` — the reference, `llama-completion` | `a city in the same country. The city is in the` |
| `mine` — engine unstated, **what the check did** | `a city in the same country. The city is in the` |
| `mine` — `--engine stand-in`, **what the check meant** | `the city of Paris. The city of Paris is the city` |

The first two are **byte-identical**, because they are the same implementation
reached by two paths. The third — MCF's own engine, the thing the oracle exists
to check — says something else entirely. So on this model the comparison the
oracle is for was being replaced by a tautology that cannot fail, and the
divergence it should have surfaced is real.

**What is still not established.** Whether any *past* oracle run was actually
vacuous. That needed a healthy provisioned daemon up at the time, and the runs
whose output survives do not show one. What is established is that the
mechanism is real and reproducible on demand, rather than a hazard reasoned
about from the source.

**But the run whose output does survive shows the other half of the defect.**
On 2026-08-28 the oracle ran with a *wedged* daemon — one accepting connections
and never answering (B-182). Every model failed the generation section's gate,
which was a silent `|| continue`. The section printed **nothing at all**, and
the summary read:

```
the oracle: MCF and the reference agree on all 142 comparisons
```

Those 142 were the *distribution* comparisons, which go through
`examples/margins.rs` — a binary that links MCF's engine directly and cannot
reach a daemon, so that half was never at risk. The generation comparison ran
**zero times** and nothing in the output said so. A4 requires that what was not
done be said rather than counted as done; the corpus check does this correctly
(*"12 of 16 entries are not held here and were not checked"*) and the oracle did
not.

**Three fixes, and a check that covers the gap F47 named.**

1. Every `mcf run` in a scheduled tier names its engine. The oracle pins
   `MINE_ENGINE=stand-in`, because *MCF's own engine against an independent one*
   is the whole claim.
2. The generation gate counts and prints what it skipped, and the summary
   reports it.
3. `checks/tests/no_tier_inherits_an_engine.rs` holds all of it from outside.
   F47's guard watches **Rust** sources for calls that reach for ambient state
   and cannot see a shell tier invoking the built binary — there is no call to
   see, and `mcf run` consulting a daemon is *correct* for a person at a
   terminal. The defect is that a check invoked it the way a person would, and a
   check is not a person. Both negative controls were exercised: unpinning the
   oracle's `mine` fails two tests by name, and changing the corpus's engine
   fails the third.

**The pattern, now three deep.** F46 found it in a Rust test, F47 fixed that one
and built a guard for its own kind, F102 found it in a shell tier, and F103
finds it in the shell tier that matters most. Each time the rule already existed
— §3.12 has said since the beginning that a suite whose answer depends on the
state it found is not a suite — and each time the guard did not reach the new
ground. The lesson is not *write the rule down*; it is that a guard covers the
shape of the place it was written for.

**A hazard in the procedure, not the code.** The first attempt to verify this
edited `check-oracle.sh` *while that script was executing* under the exclusive
window. Bash reads a script incrementally from a file offset, and rewriting the
file in place leaves the running shell reading from the same offset into
different bytes — it can execute misaligned text, and nothing in the output
would look wrong. That run was discarded rather than read. The tiers here are
shell scripts and the natural workflow is *fix the script, then re-run*: the
fix must land before the run starts, or the run is not evidence.

**What this did not establish.** Whether `check-seed-set.sh`, `check-from-scratch.sh`
or the remaining tiers inherit anything else — a store, a socket, a record path
— which is the same audit one level further out. And nothing about whether the
oracle's *past* verdicts were wrong: the distributions half was sound
throughout, and the generation half is the one whose history cannot be
reconstructed from what was printed.

## 104 · F104 — The tier named the engine and still asked another binary, and the account could not tell (B-391, F103, F102, F93, §3.12, A19, A6)

**Found by finishing the audit [F103](#103--f103--the-oracle-could-compare-the-reference-with-itself-and-a-section-that-compared-nothing-read-like-one-that-passed-b-368-b-370-f102-f47-a19-a4-312)
left open**: *whether the remaining tiers inherit anything else — a store, a
socket, a record path.* They inherit the socket, and naming the engine does not
fix it.

**The reasoning that turned out to be half a fix.** F103 pinned `--engine
stand-in` in every scheduled tier so that a listening daemon could not answer
with a provisioned llama.cpp. The daemon does honour it —
`mcf_serve::generation::choose_engine` takes `Some("stand-in")` and returns
MCF's own engine, which was read rather than assumed. But *whose* stand-in? The
daemon's. A daemon is another process running another binary, started whenever
it was started, from whatever source was in the tree then. A tier builds a
binary, and then asks a different one.

**Measured, with a negative control.** Two release binaries differing only in
one match arm, so their digests differ and their behaviour does not. `C` is the
client; `D` serves.

| | engine line the client printed |
|---|---|
| `C` alone, no daemon | `MCF's own stand-in, build 0.1.0-m0` |
| `C` with `D` listening, `--engine stand-in` | `MCF's own stand-in, build 0.1.0-m0` |

Identical — and the second was produced by `D`. That was established by
building a marked `C` whose account said `[MARKED-B]`: with the daemon up the
mark did not appear, and with it killed the same command printed it. The engine
line is the same either way because it carried the *version*, and F93
established a year of this project ago that the version is the same string on
either side of a change to an instrument. `mcf_core::engine::Run::at_build`
exists precisely because "an engine that changes silently colours every
measurement taken after it" — and both of its callers were handing it
`BuildIdentity::current().version`, a constant.

**What it cost, measured on the corpus.** With a daemon started from `D`
listening on the inherited `XDG_RUNTIME_DIR`, `scripts/check-corpus.sh` reported
*every entry held here did what it says* — sixteen of sixteen — and `D`'s
record held **sixteen `generated` entries**. The tier had built a binary, run
sixteen models through somebody else's, and said nothing. With the isolation in
place the same command passes and `D`'s record holds **zero**. That is the
whole finding in one pair of numbers.

**Two fixes, because either alone leaves the other half.**

1. **A tier brings its own socket.** `tier_private_runtime_dir` in
   `scripts/lib-tiers.sh`; `check-corpus.sh`, `check-oracle.sh` and
   `check-online.sh` export it. No daemon is found, so the binary the tier just
   built is the binary that answers. The Rust tier has done exactly this since
   it was written — `crates/mcf-cli/tests/whole_system.rs` gives every process a
   machine of its own, *including its socket*, with the reason in a comment —
   and the shell tiers handed it the operator's.
2. **The account names the build.** `mcf_core::build_identity::identifier()`
   returns `0.1.0-m0+3c4733313727`: the version and twelve characters of the
   binary's own digest, `+unknown` where the platform will not let a binary read
   itself (A7). It is what `Run::at_build` is given and what the engine line
   prints, in this process and in the daemon. Re-run of the experiment above,
   after the fix: `C` alone prints `+3c4733313727`, `C` with `D` listening
   prints `+3225ba1cd420`. The difference is now visible to anyone reading the
   output, which is what A6 asks of a condition.

**Where a Unix socket path length became a scientific detail.** The first
attempt put the private runtime directory under this session's scratch path, 96
bytes long; the daemon refused to start because `sun_path` is 108 and MCF
appends `/mcf/control.sock`. The refusal was honest and correctly classified.
The lesson is in the helper: it computes the room it has and falls back to
`/tmp`, because a tier that isolates itself only where the path happens to be
short is a tier that isolates itself on some machines and not others — which is
the same class of defect as the one it was written to fix.

**The guard, and its three negative controls.**
`checks/tests/no_tier_inherits_a_daemon.rs`. It derives the set of daemon-reaching
subcommands from the CLI source — every module that calls
`crate::serve::socket_path()` — rather than listing them, so a new one fails the
build until somebody writes down which subcommand it is. Removing the corpus's
isolation fails it by name; deleting a known module from its table fails the
derivation test; making `identifier()` return the bare version fails the
cross-check against `sha256sum`.

**What this did not establish.** Whether any past corpus or oracle run was
served by a stale daemon: the output never said, which is the defect, and no
record of those runs distinguishes them. The record's own build identity has
carried the digest since F93, so a *record* entry can be attributed — but the
tiers print rather than record, and F83's lesson has a second instance here.
`scripts/frontier.sh` is left as it is on purpose: it is not a check, it cannot
fail, and it *asks for* a daemon by name — its numbers now say which build
answered, which is what it needed and did not have.

**What remains unaudited, one level further out again.** The record path. Every
tier that runs the binary writes generation entries into the operator's real
record, where they are indistinguishable from an operator's own runs. That is
not a wrong *answer*, so it is not this finding; it is a question about whose
record a check's traffic belongs in, and it is written down here rather than
guessed at.

## 105 · F105 — A25's guarantee was structural and unused: 4 047 record entries held content, and the export said they did not (B-392, A25, A1, A24, §6.8, F68, F104, F103)

**Found by looking at the record while auditing something else.** F104's work
ended in the operator's journal, checking which build had produced which
generation. Every `generated` entry carried a `text` field, and the text was
what the model had said.

**A25 is absolute and it says both halves**: *prompt and completion content
lives in a different store from the system record.* The structure to hold it
exists — `mcf_record::content` defines a `Content` newtype that will not print
itself and a `ContentStore` that is a different type from the journal, and
`checks/tests/content_is_not_the_record.rs` reads both modules to prove neither
can name the other's types.

**And the store had no way to hold anything.** `ContentStore` could be opened
and asked where it was. There was no `keep`, no read, no path in or out. So
nothing was ever put in it, and every completion went into the journal as an
ordinary string, where a type that never appeared could not stop it. The
guarantee was intact, checked, cited in four modules, and had never been on the
path any content took.

**Measured on the machine it was found on**, counted from the journal:

| | entries |
|---|---|
| entries in the record | 4 152 |
| **holding content** | **4 047** |
| — a model's completion, `body.text` | 4 018 |
| — an operator's prompt, `body.method.prompt` | 29 |

**And the surface said the opposite.** `mcf export` wrote the file and printed,
over it:

```
  no prompt or completion content, by construction: this reads the record and the
  record is not the content store (A25)
```

The file held 4 018 model completions. That sentence is the failure A1 is
about — not a wrong number, but a claim of a guarantee, printed at the moment
the operator decides whether a file is safe to hand to somebody.

**The guard that was there, and the half it covered.** `mcf_record::export`
does not assert its header: it computes `contains_user_content` by walking a
list of paths, and `checks/tests/a_bundle_says_what_it_holds.rs` exists to keep
that list complete. Both were written by
[F68](#68--f68--the-bundles-header-said-it-held-no-user-content-while-carrying-the-prompt-b-211-pr2-a24-a25-ii),
which found the *same claim* false for the *same reason* one field earlier: the
header was a constant `false` on the reasoning that *this module reads the
journal, and the journal is not the content store*, and a comparison had begun
recording the prompt. F68 fixed the constant, named the two paths operator text
took, and built the check that keeps the list complete. The list named
`body.method.prompt` and `body.prompt`; it never named `body.text`, because the
whole guard was framed as *text the operator wrote* and A25's sentence has two
clauses.

**So this is the second occurrence, and the first one is why it was invisible.**
After F68 the claim was computed rather than asserted, the check existed, and
`contains_user_content` came out `false` — which now looked like an answer
somebody had verified rather than a question nobody had asked about the model's
half. A guard that has been through one correction reads as trustworthy, and
that is what made 4 018 completions comfortable where they were. So bundles carrying four thousand model
completions answered `contains_user_content: false` — computed honestly, from a
list that was asking about the other half. It is F103's lesson again: a guard
covers the shape of the place it was written for.

**The fix, and why each piece is not a filter.** B9 names the violation shape —
one store with a flag and an export query that excludes it — so nothing here
removes content on the way out.

1. **The store can hold things.** `keep` and `disclose_kept`, one file per key,
   filed under the record entry's own identifier as an opaque `&str`, because
   taking an `EntryId` would give the content store a path to the record. The
   pointer goes one way: an entry says how many bytes were said and never where
   they are, so a reader with the record has no route to the content.
2. **A generation has two halves that cannot be confused.** `Produced` carries
   the account and what was said; the record gets the account, and
   `on_the_wire` adds the text and the identifiers for the caller who asked.
   The join goes one way and lives in one function. The record keeps
   `text_bytes` — a measurement *about* content, which the content module's own
   documentation had already argued belongs there.
3. **A comparison keeps its prompt's length and digest**, which is enough for
   A8 to refuse a confound and is not the text. The digest identifies and does
   not conceal: anyone who guesses `Once upon a time` can confirm it, and the
   record does not claim to hide the prompt — it claims not to hold it.
4. **A bundle still reproduces its claim.** The prompt is disclosed
   deliberately, at one call site, into a file *beside* the bundle, and the
   report names it. Two files, because `mcf_record::export` must stay unable to
   reach content or the guarantee becomes a filter again.
5. **The export tells the truth about what it wrote.** It counts the entries
   holding content and says so. On the machine this was found on it now prints
   *4 047 of those entries hold a prompt or a completion, recorded before MCF
   kept content out of its record* — a number that agrees with an independent
   count of the journal.

**What is not done, deliberately.** The 4 047 entries are not rewritten. A1
forbids a record editing its own history, and a record that edited itself to
look better would be worth less than one that says what it holds. They stay,
they are counted, and every export of them says so.

**A category the taxonomy did not have.** Filing content gives the store a
*read*, and a read has a failure that had no code: content that is there and
will not open, which must never come back as the same answer as content that was
never kept — `None` is what an entry written before this says. `record.content.unreadable`,
added with its laboratory scenario in the same change (A13), along with the
refusal of a key that is not a plain name — §3.7's traversal, pointed inward.

**Checks, and their negative controls.** The primary one is end to end rather
than textual: `whole_system.rs` runs a model through a real daemon and looks in
both stores — the record must hold a length and no text, and the content store
must hold exactly what the caller was shown, because A1 forbids fixing a leak by
losing the data. Putting `("text", …)` back into the account fails it by name.
`a_bundle_says_what_it_holds.rs` now asks about both halves of A25 and refuses a
surface that starts writing content into an entry again.

**What this did not establish.** Whether any bundle or export carrying those
entries was ever *sent* anywhere — nothing on this machine records that, which
is correct: A24 makes sending an act the operator takes, and MCF is not watching
them take it. And nothing about the other stores a tier inherits, which is where
[F104](#104--f104--the-tier-named-the-engine-and-still-asked-another-binary-and-the-account-could-not-tell-b-391-f103-f102-f93-312-a19-a6)
left off — the record path itself, which is the audit one further level out
again.

## 106 · F106 — A probe that asks for a shape, and the four whose results were never written down (B-054, B-386, D42, A1, A7, A9, F101, F103, F105)

**B-054 built: the probe that asks a model for a shape.** One JSON object,
three fields of three different kinds — a string, a number, a boolean — about a
sentence put in front of the model so that no world knowledge is needed. Four
mechanical questions a parser answers: did an object come out, does it parse, is
every field there, is each of the kind asked for. The values are deliberately
not read for sense: whether the model counted the words is a capability a
laboratory grades, and a probe that failed a model for miscounting would report
*cannot produce JSON* about a model that produced JSON.

Three framings, because how a shape is described is MCF's own choice (D46) and
one framing deciding the answer would make this a measurement of the framing.
They differ along the axis a framing can be wrong along — how much of the answer
is shown: described in words, a schema, an example filled in.

**Its first real model rewrote one of its outcomes, exactly as F101's did.**
Qwen3-0.6B-Q8_0 through the provisioned engine, five trials a framing:

| framing | conformed | departed | *first reading* | *what it was* |
|---|---|---|---|---|
| described in words | 0 | 0 | 5 produced no object | **5 still going when the budget ran out** |
| a schema | 5 | 0 | — | — |
| an example filled in | 0 | 0 | 5 produced no object | **5 still going when the budget ran out** |

All ten. Not one trial was the model declining to produce a shape; every one was
MCF stopping it at two hundred tokens while it was still writing. The first
reading — *0 conformed, 5 produced no object* — reads as a fact about the model
under two of three framings, and it was a fact about the budget.

`Attempt::Unfinished` is a variant now, decided by *how the turn ended* rather
than by what came out: a turn the model itself finished with no object is an
observation about the model; a turn the budget cut short is A7's *not within
this many*. This is the third time the same shape has cost something here —
F101's probe folded *called without the wrapper* into *did not call*, and this
one would have folded *interrupted* into *declined*.

**And then the budget was measured rather than argued about.** The same probe,
same model, same engine, at two hundred tokens a trial and at eight hundred:

| framing | at 200 | at 800 |
|---|---|---|
| described in words | 0 conformed, 5 unfinished | **5 conformed** |
| a schema | 5 conformed | 5 conformed |
| an example filled in | 0 conformed, 5 unfinished | 0 conformed, **5 finished with no object** |

The first row is a constant that was measuring itself: two hundred tokens was
not enough for this model to finish an object it *could* produce, and the probe
would have reported a framing that works as one that does not. The third row is
what a large enough budget buys — at two hundred it was *interrupted*, and at
eight hundred it is an observation: this model finishes its turn and produces no
object when it is shown an example. The shipped budget is eight hundred, for
that reason and with that measurement beside it.

**And the thing found while wiring it in.** B-386's row says *a probe's outcome
is written to the record, not only printed*, done, with the check
`a_probe_is_written_down.rs` behind it. It held for one probe of four. The check
read `fn context_lines(` **by name** and asked whether that function wrote — so
the chat-template, stop-condition and tool-calling probes printed their results
and kept nothing, and B-386's own done-when — *a probe run yesterday can be read
back today* — was false for three quarters of the probes while the row said it
was true.

That is the same defect as
[F103](#103--f103--the-oracle-could-compare-the-reference-with-itself-and-a-section-that-compared-nothing-read-like-one-that-passed-b-368-b-370-f102-f47-a19-a4-312)
and
[F105](#105--f105--a25s-guarantee-was-structural-and-unused-4-047-record-entries-held-content-and-the-export-said-they-did-not-b-392-a25-a1-a24-68-f68-f104-f103),
three days running: **a guard covers the shape of the place it was written
for.** Each of the three guards was carefully built, correct about its instance,
and blind to the next one — and in all three cases the row above it said the
rule held.

**So this guard is bound to the shape.** Any function in `probe.rs` that renders
an `Outcome::Observed` must record it, and the number of such renderers must
equal the number of `Method` constants the probes define. A probe cannot now be
added, run and printed without the check seeing it. The chat-template probe was
rendered inline in the command, which is how it stayed invisible to a check
looking at functions; it has a renderer of its own now.

**And the write goes before the verdict**, checked by position rather than by
promise. A record written inside the branch that decides what an observation
*means* is a record of the interpretation: `agrees` is as much a measurement as
`diverges` (A9), and the way that goes wrong is not a missing write but a
conditional one.

**Both negative controls fire**: removing a renderer's write fails by name, and
a `Method` with no renderer fails the count.

**What this did not establish.** Whether the tool-calling probe has the same
`NoCall`-versus-unfinished conflation the structured one had. It reads the same
`Trial` and does not consult it, so the shape is there; what is missing is a
model that shows it, and the one this ran on called well within its budget under
the framing that worked.

## 107 · F107 — The oracle's first disagreement in three days was the instrument's, not the engine's (B-393, B-368, B-373, F27, F34, F103, A19, A5)

**The first full oracle run after [F103](#103--f103--the-oracle-could-compare-the-reference-with-itself-and-a-section-that-compared-nothing-read-like-one-that-passed-b-368-b-370-f102-f47-a19-a4-312)
pinned `mine` to MCF's own engine** reported *1 of 342 comparisons disagreed*:
`Qwen3-4B-Q4_K_M`, on *The capital of France is*, **differs with room to
spare** — at step 5 MCF's margin was 0.70982, over the 0.40 threshold F27
measured, so not a near-tie.

```
MCF:       Paris. The capital of Germany is Berlin. The
reference: Paris. The capital of Belgium is Brussels. The
```

**It was not the engine.** The reference's own distribution at that step, from
`llama-server`'s `n_probs`, reproduced by hand:

| token | reference | MCF | difference |
|---|---|---|---|
| ` Belgium` | −1.0522 | −1.4194 | −0.367 |
| ` Germany` | −1.0721 | −0.7096 | **+0.363** |
| ` France` | −1.7574 | −1.9801 | −0.223 |
| ` Paris` | −2.4918 | −2.5118 | −0.020 |

The reference's own top-two gap is **0.0199**. The model is indifferent between
Belgium and Germany. MCF's logits are shifted by about a third of a logit —
inside what quantized arithmetic does, and the two moved in *opposite*
directions, so the contrast swung by 0.73 and the tie became a clear win **in
MCF's ranking**. The margin measured that win.

**The better instrument was running beside it and disagreed.** The
distributions section compared the same model, the same prompt and the same
step in the same run:

```
Qwen3-4B-Q4_K_M.gguf  distributions on The capital of France is at step 5:
                      top20 0.433  top5 0.367  kl 0.0520
```

KL 0.0520 against a floor of 0.20, and under the 0.113 maximum F34 measured for
a clean engine. Both instruments ran, both printed, they said opposite things
about one step of one generation, and the weaker one decided the verdict and
failed the check. Nothing noticed, because nothing compared them.

**Why the margin is the wrong quantity.** It is *MCF's own* gap between its
first and second choice. F27 built it to answer *was the model indifferent
here* — and it answers that about **MCF's arithmetic**, not about the model.
The two coincide only when the engines agree; where they disagree, which is
exactly when the question is asked, MCF's margin describes the ranking MCF
produced. Every one of F27's four coin-flips had the reference's choice as
MCF's runner-up *and* a small MCF margin; this case has the first signature and
not the second, which is the combination F27's evidence could not distinguish
because it never occurred below 1.7B.

**The fix, and what it does not do.** A generation divergence is now recorded
rather than counted, and resolved by the distribution comparison at the same
step: agreeing distributions mean the two engines agree and the decision was a
tie on the reference's side, and the resolution prints the reference's own
top-two gap — the number that actually says *coin flip*. A divergence the
distributions never reached is still a disagreement and says that it rests on
MCF's margin alone (A5, applied to a check's own confidence). The threshold is
unchanged: it is a fine first filter, and F27's calibration of it stands. What
changed is that it no longer has the last word when a better instrument has an
opinion.

**A hazard in the procedure, recorded beside F103's.** Waiting for the long run
to finish was done with `until ! pgrep -f "check-oracle.sh"; do sleep; done` —
and the shell running that loop has `check-oracle.sh` in its own command line,
so `pgrep` matched the watcher. It reported the oracle still running for an hour
after it had finished, twice. It is the session's own small instance of the
thing every finding above is about: an instrument that measured itself and
reported the answer as though it were about something else. Match on something
the watcher does not contain.

**What this did not establish.** Whether MCF's third-of-a-logit shift at this
position is ordinary quantized arithmetic or a small real defect that the KL
floor is too loose to see. The floor was measured against a clean engine and two
gross defects (F34); nothing has yet measured what a *subtle* one looks like at
4B. That is a question about the floor, and it is not this.

## 108 · F108 — Two rules rested on somebody remembering, and one identifier had been cited four times with nothing behind it (B-394, C5, C6, B16, F80, §7.30)

**Chosen by asking which rules a machine does not hold.** B16 asks for the
machine-checked form of every rule and names the number to drive down: how much
of the discipline depends on somebody remembering. Fifteen rules rested on
review alone. Two of them — C5, *identifiers are stable for life*, and C6,
*nothing is deleted* — are the same promise from two sides, and both are about
namespaces that have already left this machine's control.

**What they protect is not cosmetic.**

* A **taxonomy code** appears in the record, in an export, and — once §XIV ships
  — in another machine's copy of this one's evidence. Renaming one silently
  reinterprets every record that carries it.
* An **entry kind's position** in `EntryKind::ALL` is what the derived index
  stores. The list says so in a comment: *appended rather than sorted in*.
  Reordering it does not fail; it reinterprets every entry ever written.
* A **register identifier** is what a finding, a rule and a commit message cite.
* A **format version** that moves is a schema change, which §7.30 makes an
  interface event rather than an edit.

None of these can be renamed by a careful edit. They can only be renamed by an
edit nobody noticed — which is precisely the class of thing a review does not
catch and a ledger does.

**The mechanism.** `checks/identifiers.tsv`: 641 lines, one per published
identifier, with the meaning where the identifier has one and the position where
the position is the interface. The check recomputes the set from the source and
the documents; an identifier in the ledger and not in the tree is a removal or a
rename, and one in the tree and not in the ledger is an addition that fails until
somebody writes the line. That friction is the mechanism, not a side effect: the
ledger's diff is the only place a reader can see what this project has promised
to keep.

**Three negative controls, each firing by name**: two kinds swapped in
`EntryKind::ALL` (reported as the two positions that moved), a code's meaning
rewritten (reported as the old meaning gone), and the bundle format version
bumped from 1 to 2.

**And the citation half found a live one.** `every_identifier_citation_resolves`
resolved rules, resolutions, laboratories, milestones and proposals — and not
findings, the namespace every evidence-bearing decision cites. Adding `F` to it
found that **`F80` had been cited four times and its section had never been
written**: twice in this file, once in its own table of six defects, once in the
register's changelog. The commit that fixed the defect said *F79, F80* and wrote
only F79.

[F80](#80--f80--the-registers-headline-was-wrong-by-five-items-and-the-check-was-green-because-it-skipped-what-it-could-not-parse-b-383-c5-a1-a2-35)
is written now, from that commit and the register's Version 193 entry, and marked
as reconstructed. Its own subject is *a check that passes while failing to do its
job* — the shape F102, F103, F105 and F106 each found somewhere else — so a
dangling citation to it was, for a day, a small instance of what it describes.

**What this did not establish.** Whether any *earlier* citation resolved to
nothing and was quietly fixed by renumbering. Git would answer it and the ledger
begins today; from here a removal fails the build, which is the property that
was wanted rather than an audit of the past.

**Thirteen rules still rest on review alone**, and the honest ones among them —
A1, *never lose information* — are not reducible to a mechanism. B16 asks for the
number to fall, not to reach zero.

## 109 · F109 — A rule of thumb that cannot be read as a result, cannot be recorded, and expires when the laboratory lands (B-380, DEC-002, A21, A25, §3.15, §3.18)

**The item said the hazard is the whole of the design**, and it is: a rule of
thumb printed beside a measured number *in the same typeface* becomes a measured
number to a reader who is not looking for the difference — and the readers
touchstones exist for are exactly those readers. MCF already has a shape for
this. A21 gives a model's own claims three states, *declared*, *verified*,
*unknown*, and forbids a fourth. Turned on MCF's own sentences, a touchstone is
a fourth thing that is none of them: a statement about the world this machine
has not measured.

**Three structural properties, not three conventions.**

1. **No digit, anywhere in a touchstone.** It describes a *relation* in words;
   every number a reader sees came from a measurement, so a number in a
   guidance sentence is a number somebody is entitled to check and cannot.
2. **It cannot exist without its limits.** The constructor requires what MCF has
   *not* measured about the relation, and no rendering omits it — there is one
   `Display`, it is prefixed `RULE OF THUMB, not a result`, and it always ends
   with *MCF has not measured …*. A surface that wants the words alone has to
   call `bare()` and can be found by name.
3. **It cannot reach the record.** No module of `mcf-record` may name the type.
   This is A25's reasoning transferred exactly: a filter can be misconfigured,
   and a sentence that was never written cannot be read back as data by somebody
   who has forgotten where it came from. F105 is what happens when that property
   is asserted and not held.

**And it expires.** Each touchstone names the register item whose laboratory
would replace it with a measurement — `B-110` for what a smaller quantization
costs in quality, `B-122` for what a person notices, `B-205` for a workload
unlike this one. When that item is done, offering a rule of thumb about
something MCF can now measure is the defect, and the check fails until the
touchstone is removed. That is B-380's *a laboratory that measures one replaces
it and the replacement is visible as a change*, held by a machine rather than by
whoever remembers. Both halves were exercised: a digit added to a touchstone
fails by quoting it, and marking `B-110` done fails by naming the subject MCF
could now measure.

**Where they appear, and why not everywhere.** The operator's answer to DEC-002
put the comparison view first, on the reasoning that a number teaches by
contrast — *0.77 tokens per character means nothing alone and everything beside
2.00* — so a reader in front of two values *sees* the difference the touchstone
describes instead of being asked to believe it. Which ones appear is chosen by
what the comparison isolated and what verdict it reached: a rule of thumb about
quantization beside two unrelated models is a sentence about something the
reader is not looking at, and a guidance section stops being read the moment it
contains things that do not apply.

**Demonstrated on a busy machine, which was not planned.** The comparison used
to check the rendering ran while another project held this machine's exclusive
window: 113 paired trials with 26 of 32 cores competing. MCF's own report says
so — *NOT FIT TO CONTRIBUTE — 88.3% of this machine was already busy, OUTSIDE
the band of 30.0%* — and the touchstones sat below it under their own rule. Two
marks on one screen, each about a different thing: one says this measurement
cannot travel, the other says this sentence is not a measurement. Neither
diluted the other, which was the risk. It is also B-217's band firing on real
contention rather than the synthetic load F95 measured it under.

**What remains.** `mcf explain`, which the operator put second. And the
catalogue is deliberately three: every entry is something MCF says on no
evidence of its own, and §3.15's rule about choices the reader cannot see
applies twice as hard to sentences the reader cannot check.

## 110 · F110 — A probe for each modality, or a reason: what a language costs, whether an artifact embeds, and three declinations in writing (B-057, D42, F81, F106, A7, A21, §X)

**B-057's done-when has two halves and the second is the honest one**: *each
in-scope modality has a probe; each out-of-scope one is recorded as declined.* A
modality MCF simply does not mention is one a reader assumes it checked and
found nothing wrong with — the same silence A7 forbids about a model's own
metadata, pointed at MCF's coverage.

**D42's test, applied to each rather than a probe written for each.** *A probe
earns its place when a wrong answer to it would corrupt a measurement or a
served answer*, and it must be answerable by observation rather than by
judgement.

**In scope, and built.**

*`language-cost`* asks the **file's vocabulary** and no engine at all — which is
unusual enough that the conditions say so rather than naming an engine that did
not participate. The same sentence in six languages, counted: exact,
deterministic, milliseconds. It earns its place because tokens are the unit of
the context budget, the token budget, the time a turn takes and what a
comparison holds still, so a model that spends two and a half times as many on
one meaning is being asked a different question at the same budget. Measured on
three files:

| | English | Japanese | Arabic |
|---|---|---|---|
| SmolLM2-135M | 16 | **52** (325%) | **46** (288%) |
| gemma-3-270m | 16 | 20 (125%) | 22 (138%) |
| all-MiniLM-L6-v2 | 16 | 22 (138%) | 49 (306%) |

F81's care is kept in the sentence itself, because that is where it is lost: a
cost is a fact about a *vocabulary*. A model can be excellent at a language its
vocabulary spells expensively.

*`embedding`* asks for one vector twice and reports the width and whether the
two were identical **to the last bit**. Not *near enough*: MCF's own engine does
the same arithmetic in the same order, so anything but equality is a defect
rather than noise, and a tolerance there would hide it. On all-MiniLM-L6-v2 the
file declares bert and width 384, and 384 came back, twice, identically —
declared and verified agreeing, which is as much a result as a divergence (A9).

**Declined, in writing, each with what MCF looked for.**

* **Vision** — looked for the `clip.*` metadata and projector tensors a
  multimodal file carries. MCF's own engine implements text transformers and no
  engine here is driven with an image, so a probe reporting *no vision* would be
  reporting MCF's reach as a property of the model, which is the confusion A21
  exists to prevent.
* **Multilingual fluency** — a graded task. What a language *costs* is probed;
  how well the model speaks it needs a rater, and reporting the first as the
  second is exactly the reading F81 was written to prevent.
* **Reasoning modes** — what is observable is that a model emitted its declared
  thinking markers and how many tokens it spent inside them. That is worth
  asking and is not a modality: it changes the token budget a turn needs, which
  makes it a *configuring* probe with its own row. [F106](#106--f106--a-probe-that-asks-for-a-shape-and-the-four-whose-results-were-never-written-down-b-054-b-386-d42-a1-a7-a9-f101-f103-f105)
  measured the cost of not having it — ten trials of ten spent a whole
  two-hundred-token budget before producing anything.

**And the first run recorded a condition that did not participate.** The
embedding probe was wired with the same engine string as its neighbours, so its
conditions read *provisioned llama.cpp, through the daemon* about arithmetic MCF
performed in its own process — `mcf embed` consults no daemon, which F103
established rather than assumed. Caught by reading the output of the first real
run, which is the fifth time in four days that a condition naming the wrong
engine has turned up. It names the in-process path and this binary's build now.

**One guard improved by being wrong about its own subject.** The check that
requires every probe to record what it observed reads the `Method` constants to
count probes — from a list of three named files. Two probes arrived in two new
files and the list was still three. It reads the directory now: the same lesson
F103, F105 and F106 each paid for, met once more while adding the thing that
triggered it.

## 111 · F111 — The budget tier fired on its first run in two days, and most of what it caught had been there for one of them (B-011, B20, D24, B-185, B38)

**What happened.** The exclusive window came free after four days of stale
tiers, the performance budget ran, and it failed on its first figure:

```
core binary, no engines   4635304 B (4.4 MiB) — within
  before 4054184 B, after 4635304 B (+14.3 %)
FAILED: regressed against a tolerance of 2.0 % (B20, B-011)
```

Two things are true at once and the report says both: the binary is **within**
D24's ceiling of 40 MiB, with an order of magnitude to spare, and it has grown
14.3 % since the last recorded reading. The ceiling is the promise; the
tolerance is B20's *no silent regression*, and it is the one that fired.

**Attributed rather than re-baselined on a guess.** The binary was built at each
first-parent merge since the baseline and measured:

| merge | bytes | Δ |
|---|---|---|
| the era the baseline was taken in (27 Aug) | 4,094,000 | — |
| the measurement work — the benchmark and its findings (28 Aug) | 4,564,688 | **+470,688** |
| B-366, threads | 4,574,656 | +9,968 |
| B-384, B-053, B-182 | 4,593,456 | +18,800 |
| B-391, B-392, B-054 | 4,634,552 | +41,096 |
| B-394, the identifier ledger | 4,634,552 | 0 |
| B-380, touchstones | 4,637,576 | +3,024 |

**Eighty per cent of the growth landed on 28 August**, in one merge, and the
four merges of the day this fired account for 63 KB between them — 1.4 %, under
the tolerance. The sections say it is code rather than a blob: 3.78 MB of
`.text` against 562 KB of `.rodata`, with nothing anomalous in either.

**So the finding is not the growth. It is what the staleness cost.** The tier
that would have caught this in one merge did not run for a day and a half, and
in that interval the thing it watches moved seven times its tolerance. A stale
tier is not a neutral state — it is a **growing blind spot**, and the size of the
blind spot is exactly what accumulated while nobody looked. B-185 already
refuses a *release* on a stale tier and `ci.sh` already prints the ages on every
run; what neither could say is what the debt was worth, and now one instance of
it has a number.

**And the reason it was stale is worth writing down too**, because it is not
carelessness: every heavy tier takes this machine's exclusive window, and for
most of the day the window was held by other projects — one holding for
thirty-seven minutes with two more queued behind it. The discipline that
protects a timing measurement from a busy machine is the same discipline that
delays the tier which measures it. That is a real cost of B35's window and not
an argument against it; the answer is to run the tiers when the window frees,
which is what happened here.

**The re-baseline, with its reason**, follows the precedent B-011's row set the
last time this happened: a re-baseline naming the features that account for the
growth, never a tolerance widened. What accounts for it is the measurement work
of 28 August — the paired comparison machinery, the effect-size interval, the
seed set and the instrument cross-checks — plus this day's probes, content
store and touchstones.

**A trap found in the re-baselining itself.** The number written by hand was
4,666,680 B and the tier's own passing run recorded 4,664,408 — 2,272 bytes
apart, at the same commit, from a build that is reproducible byte for byte
(B-001). The difference is `ci.sh`'s `--remap-path-prefix`, which shortens the
paths embedded in the binary: **the artifact the tier measures is not the one
`cargo build --release` produces at a terminal.** Both readings are of real
artifacts and the baseline file records the conditions of the one it holds, so
nothing here is wrong — but a person re-baselining by hand is measuring a
slightly different thing than the tier will, and only the 2 % tolerance hid it.
The tier's own measurement is the authority and it overwrote the hand-written
one on the passing run, which is the right order.

**What this did not establish.** Whether 470 KB is a *reasonable* price for that
body of work. Nothing here measures what a feature ought to cost, and inventing a
figure to compare against would be exactly the invented intent A23 forbids. What
is established is what it did cost, when, and that the artifact remains an order
of magnitude inside the ceiling it promised.

## 112 · F112 — Two absolute rules named checks that did not exist, and one of them had a hole on a shipped surface (B-073, B-072, A6, A22, B16, D27)

**Chosen by asking the rules rather than the roadmap.** Six *absolute* rules
cite a check that names a backlog item, and six of those items are not done. Two
of the six are merely open rather than blocked, which makes them buildable
today: A6's *no number without its conditions, its sample count and its spread*
(B-073) and A22's *the headless path can do everything* (B-072).

**A6's hole was real, and on a surface an operator reads.** `Measurement<Q>` has
no constructor that omits a condition set and no rendering that drops one —
B-005 made that structural and a check holds it. But a surface never had to use
that rendering. It could ask for a percentile, get a bare `Q` back, and print it.
`mcf doctor` did:

```
cold start to first command response   p99 960005 ns over n=100 — not attributable
```

A value and a sample count, assembled from two separate asks, with **no spread
at all** — and the median printed on the next line only because somebody had
written it there. The rule was held for the whole measurement and by nobody for
the statistic taken out of it.

`Stated<Q>` is what a statistic leaves in now: one `Display`, and it carries the
statistic's name, the value, `n`, the median, p5–p95, the minimum and the
maximum. `Budget::statistic` returns one, so the bare number is reachable only
through `value()` — named for what calling it does, the way `Content::disclose`
and `Touchstone::bare` are — and a **surface** that formats that is what the
check looks for. A test may: the budget tier compares a statistic with a
baseline and writes it to a file, which is arithmetic and a record rather than a
view, and the distinction is the one A22 draws.

**Two existing checks caught the change being wrong on the way in**, which is
the part worth recording. `measurement_has_one_way_in.rs` requires every
constructor in the module to take conditions, and `Stated::new` takes a
`&Measurement<Q>` — the same guarantee one step along, since a thing built out
of a measurement cannot be built out of a bare number; the rule is amended to
say both spellings. And `an_event_class_figure_is_reported_at_the_percentile_d27_names`
failed because the first version of `Stated`'s rendering dropped the **median**,
which D27 wants beside the p99 precisely because the gap between them is what a
busy machine looks like. Neither was my noticing.

**A22's check was named and never written, and the rule said as much.** It
observes that the rule is close to self-enforcing — a capability reachable only
through an interface is one the laboratory cannot test, which A19 forbids — and
*close to* is not a check. What is enumerable today is the control plane: every
variant of `Request`, read from the enum rather than from a list, must be sent by
the command-line crate, and the three a person asks for by name must be commands
in the argument parser.

It passes today, and the hypothesis that sent me looking was wrong: I expected
`Status` and `Holding` to be answerable by the daemon and unreachable from a
terminal, and `mcf status` already asks both. Recorded because a check that
confirms a property is worth exactly as much as one that finds a defect, and
because the guess is part of the account.

**The compiler holds the other direction, and better.** Trying to write the
negative control — a new operation with no command — would not compile: the
daemon's match over `Request` is exhaustive, so an operation nobody handles is a
build failure before any check runs. The control had to add the variant *and*
handle it to reach the check at all. That is the shape B16 asks for, found by
attempting to break the thing rather than by reasoning about it.

**And the half that cannot be written yet is written as an assertion.** When
§XI's window arrives it becomes a second client of the same wire, and A22's real
target is an action *it* has that no command does. A test fails the day a
`mcf-window` crate appears, so its actions are enumerated then — rather than
somebody discovering six months later that A22 had been checked against one
client. The same reasoning put the privileged helper and reference-model
neutrality in M0: a special case is far cheaper to prevent than to find.

**Rules resting on review alone: unchanged at 13.** These two were already
machine-checked *in name*; what changed is that the checks now exist. The
number to watch is B16's, and the number this moved is a different one — four
absolute rules still cite items that are open or blocked.

## 113 · F113 — The load tier found a race in the laboratory, and it was six runs in a thousand of a file being written and executed at once (B-191, B-009, §3.17, D26, A13)

**Found by running a stale tier.** The load tier had not run in three days;
when it did, two of its six tests failed:

```
engine/server-never-listens diverged under load from what it produces alone
a scenario stopped producing its category under load
```

§3.17 is the rule behind both: a failure found once must reproduce *exactly*,
and the laboratory's own suite runs every scenario 64 ways at once to check that
it does.

**Diagnosed rather than guessed at.** The scenario was run 1,024 times across 64
workers with every outcome counted:

| runs | outcome |
|---|---|
| 1,018 | `engine.hang.no_output` — the declared one |
| **6** | `engine.spawn.refused` — **`Text file busy`** |

`ETXTBSY`, and it is not this machine being short of anything. It is the
write-then-exec race that every multi-threaded program creating an executable
has: the scenario writes its fixture server and execs it, and a **sibling
worker** that forked while that file was open for writing holds an inherited
copy of the descriptor until its own exec. Close-on-exec closes it *at* exec,
not at fork, so the window is real, it belongs to another thread, and nothing
the scenario can do from its own side removes it.

**The first diagnosis was wrong in an instructive way.** Counting the *rendered*
failures showed 1,024 identical lines and no divergence at all — because
`Display` renders the category and the sentence, and the difference was in the
failure's **context**. A comparison that reads what a person sees is not a
comparison of what was produced; the second pass compared the whole structure
and the six appeared immediately.

**The remedy is retrying past a transient race, and the number came from the
measurement.** The scenario already retried a refused spawn three times, with a
comment reasoning that a spawn which did not happen means the scenario has not
run yet — right, and three landed inside one window six times in a thousand. It
is twenty now, retried only while the refusal is that transient kind, and a
spawn refused for any other reason is still reported as the real observation it
is. After the change: 1,024 runs, **one** distinct outcome, and the tier's
135,680 scenario runs across 64 workers pass.

**What this says about the tier.** A property that holds a thousand times and
fails six is exactly what a scheduled tier is for and exactly what a gating one
cannot afford to look for. It is also the second thing in one day that only ran
because the exclusive window came free after four days —
[F111](#111--f111--the-budget-tier-fired-on-its-first-run-in-two-days-and-most-of-what-it-caught-had-been-there-for-one-of-them-b-011-b20-d24-b-185-b38)
was the first. Two of the five stale tiers had something to say the moment they
were asked.

**What this did not establish.** Whether twenty is enough on a machine busier
than this one. The bound is stated where a reader will meet it, the failure past
it is reported rather than swallowed, and what a *hundred*-way run does is not
known — which is A7's answer rather than a number nobody measured.

## 114 · F114 — MCF was keeping its own probe traffic in the store meant for the operator's private text (B-146, B-392, §6.8, A17, A25, B9)

**Found by opening the store.** [F105](#105--f105--a25s-guarantee-was-structural-and-unused-4-047-record-entries-held-content-and-the-export-said-they-did-not-b-392-a25-a1-a24-68-f68-f104-f103)
gave MCF a content store seven hours earlier. It held **614 files**, and most of
them were not the operator's:

```
generated_2026-08-29T07-19-18Z…   <think> Okay, the user is asking for the capital of France…
generated_2026-08-29T07-19-22Z…   <think> Okay, so the user is asking how many days are in a week…
generated_2026-08-29T07-19-25Z…   <think> Okay, the user is asking me to name one colour of the rainbow…
```

Those are the chat-template probe's three constant questions — `probes.rs` lines
629–631 — and a model's answers to them. MCF's own traffic, filed beside a
person's `mcf run` and indistinguishable from it.

**§6.8 names this exact conflation**, and it is the reason the rule is written
the way it is: *benchmark suites — whose content is fixture data, not user data
— may be recorded in full, and this distinction is exactly why suite data and
user traffic must be structurally separated rather than separated by
convention.* B-146 has said so since the register was written.

**Why it matters in practice, before any retention policy exists.** An operator
who purges *their* text would delete MCF's evidence with it; an operator
auditing what MCF holds of theirs meets six hundred files that are not theirs
and has to take somebody's word for which is which. DEC-005 will decide the
retention of the first category and §6.8 already permits keeping the second in
full — two different answers that cannot be given about one store.

**The split, and why it is not a flag.** Two types with no conversion —
`Whose::User`, `Whose::Fixture` — and two directories, `content/` and
`fixtures/`. A store is opened *for* a category and writes only there: `keep`
takes no category, so a caller cannot aim it. And the category **travels on the
wire**, because the daemon cannot tell a probe's constant question from a
person's prompt by looking at it — they arrive on the same socket in the same
shape. A request that does not say is the operator's, which is the safe
direction: MCF's traffic under a person's retention is untidy, and a person's
under MCF's is the privacy failure §6.8 exists to prevent.

**The compiler enumerated the callers.** Adding the field to `Request::Generate`
failed the build in seven places — `mcf run`, `mcf bench`, `cross-check`, two
probe paths, the self-cost measurement, and the daemon's own pattern — which is
the whole list of things that ask a model for anything, produced by the type
system rather than by grep.

**Demonstrated on a fresh machine.** One `mcf run` and one `mcf probe` against
an isolated data home:

| | files |
|---|---|
| `content/` — the operator's | **1** |
| `fixtures/` — MCF's own | **51** |

and the record carries it as a condition rather than as content: 52 entries say
`asked_by: fixture`, one says `asked_by: user`. Before the change all 52 would
have been one heap.

**The 614 that predate the split stay where they are.** A1 forbids rewriting the
record's history and the same reasoning holds for what sits beside it: MCF could
guess which of them are its own by matching prompts it recognises, and a guess
that moves a person's text is exactly the thing this finding is about. They are
in `content/`, they are mostly MCF's, and this paragraph is the only honest
place to say so.

## 115 · F115 — A16's fifth gate was in the rule and nowhere in the code, and every comparison MCF had recorded was uncontributable for saying nothing (B-160, B-039, A16, A24, B-203, B42, §3.20)

**Two things, found by building A24's check.**

**One: four of five.** A16 is absolute and names five categories that are always
gated — untrusted execution, large irrecoverable use, network exposure,
destruction, and *publishing anything off this machine*. Its own check says *the
five categories are enumerable in code*. Four were. The enum's comment read
**"All four, in the order §6.14 names them"**, and §6.14 does name four: the
fifth comes from §3.20, which A16 absorbs and which nothing enumerated. The
missing one is the only one of the five that cannot be undone, which is why A24
exists as its own rule.

It is there now as an **absence** rather than a gate — MCF sends nothing
anywhere, there is no destination, no address to configure and no path that
opens one — which is the strongest statement available and the weakest position:
it holds only while it is true. So the absence is *checked* rather than
asserted. `nothing_sends_anything_anywhere` walks every shipped source for a
connection to somewhere else; three sites are declared with what each is for.
Writing it found two of them, and its first version reported a false positive —
a `use` line for a type name — which is why it matches what *opens* a connection
rather than what mentions one.

**Two: nothing MCF has ever recorded can be contributed.** `mcf share` selects
comparisons out of the record, and on this machine's 70 of them the answer is
none — for two different reasons, both of which it prints:

| rows | why not |
|---|---|
| 38 | no established size: the arms did not separate, the comparison was confounded, or the size was not established at the resolution asked about |
| **32** | **the workload is the operator's own** |

The first is honest and expected: *no difference* is a real result and it is not
a size (A9, A8). The second was not. B-203 requires that the marking of where a
workload came from **travel from production, on the row** — *a marking added at
the boundary is one that can be forgotten at the boundary* — and `mcf bench`
never wrote it. So every timing comparison MCF has ever recorded was refused for
**saying nothing**, and the default that refused them was the safe direction
rather than the row's own answer.

The row says it now, and it says `custom`, which is not a placeholder: `mcf
bench` takes its prompt from `--prompt`, so the work is the operator's own by
construction today. A row produced from a workload MCF ships will say `declared`
— and that is the day this becomes a choice rather than a fact.

**And the confirmation shows the rows.** `Contribution`'s only rendering was
*2 row(s): 1 comparison(s) and 1 absolute(s)* — precisely the description A24
forbids being shown **instead of** the rows. It renders every row in full now,
with the count after them rather than in place of them, and there is no second
rendering that could drop one. The file written holds the same text the operator
read, because a file whose contents differ from the confirmation is a
confirmation of something else.

**A mistake in the verification, worth recording.** The first check that `mcf
bench` had begun writing the marking ran the command with its output discarded,
found no new row, and looked like a build that had not taken. The command had in
fact refused: a benchmark runs through the daemon and none was listening,
because the previous demonstration had stopped it. Discarding the output of a
command whose *output is the evidence* wasted three attempts — the same shape as
F113's first diagnosis, one step earlier.

## 116 · F116 — The benchmark's standard question, chosen by measuring 27 vocabularies rather than by taste (B-160, B42, F110, F81, §6.37)

**The operator's decision, and the measurement it needed.**
[F115](#115--f115--a16s-fifth-gate-was-in-the-rule-and-nowhere-in-the-code-and-every-comparison-mcf-had-recorded-was-uncontributable-for-saying-nothing-b-160-b-039-a16-a24-b-203-b42-320)
found that every timing comparison MCF had ever recorded was refused for
sharing because the row never said where its work came from — and that the
deeper cause was that `mcf bench` had no standard question at all. It required
one to be supplied, so every result was the operator's own by default rather
than by anyone's judgement.

The project already describes the answer for laboratories: *ship a standard
workload and accept a replacement; results from the standard travel, results
from a replacement do not.* The benchmark simply never had the first half. The
operator chose to build it.

**A standard question is only standard if it is the same work for every model
asked it**, and that is not automatic: the same English sentence costs 16 tokens
on one vocabulary and 52 on another once other languages are involved (F110).
Four candidates, measured across every vocabulary this machine holds:

| candidate | tokens | spread |
|---|---|---|
| *The quick brown fox jumps over the lazy dog…* | 16–19 | 19% |
| *Write a short paragraph explaining how a lighthouse keeper…* | 15–20 | 33% |
| **In three sentences, describe what happens to a river…** | **20–22** | **10%** |
| *Describe in three sentences what happens to a river…* | 16–18 | 12% |

Two tokens of variation across 27 vocabularies, which is as close to *the same
work* as a question written in words gets.

**Why this shape.** Plain English, because that is where vocabularies agree. No
markers and no numerals, which tokenize unpredictably. An instruction rather
than a fragment, because that is what people benchmark. A bounded answer —
*three sentences* — so that a pinned token budget is near what the model would
have said anyway rather than a cut across the middle of a thought.

**It is fixed for life.** A standard question that changed would silently make
yesterday's shared results incomparable with today's, which is the one thing a
standard exists to prevent. A future change is a second question beside this
one, never an edit to this one.

**What it does not settle**, and the operator said so within a minute of seeing
it: 27 vocabularies is a sample. A family MCF has never met can tokenize it
anywhere, and a mismatch between two arms is work one arm does and the other
does not — which pairing cannot cancel, because it is not noise.
[F117](#117--f117--the-pinned-length-was-declared-and-never-enforced-and-a-rate-computed-the-obvious-way-is-a-property-of-the-length-you-chose-b-081-d19-a19-34-f116)
measures what a token of mismatch is worth and what to do about it.

## 117 · F117 — The pinned length was declared and never enforced, and a rate computed the obvious way is a property of the length you chose (B-081, D19, A19, §3.4, F116)

**Found by the operator asking a question about the design**: *wouldn't more
verbose models always take longer, since they respond with more words?*

They would, and MCF knew it. The timing discipline is called `LengthPinned` and
the code says why:

> *a timing that varies because one run stopped earlier is measuring the stop,
> not the speed. A timing comparison that let its arms stop where they liked
> would be reporting the models' verbosity as the machine's throughput.*

**The pin was declared and never enforced.** The engine that does the timing
treats a token budget as a *ceiling* and stops at the model's own end-of-turn
token. And MCF could not see it happen: through that engine an answer arrives as
one chunk, so the account reads `1` token and `stopped:
unknown_the_engine_did_not_say`, while the record writes `tokens_pinned: 32` in
a form that reads as enforced.

**Measured, across a matrix of two models, five lengths, two settings of the
pin, seven repeats a cell.**

| model | asked | produced, pin off | produced, pin on |
|---|---|---|---|
| SmolLM2-135M | 8 / 16 / 32 / 64 / 128 | **5 / 5 / 5 / 5 / 5** | 8 / 16 / 32 / 64 / 128 |
| Qwen3-0.6B | 8 / 16 / 32 / 64 / 128 | 8 / 16 / 32 / 64 / 128 | 8 / 16 / 32 / 64 / 128 |

Unpinned, SmolLM2 produces five tokens **whatever it is asked for** — the length
axis collapses entirely for that model — while Qwen3 runs to the budget every
time. A comparison of the two at 128 tokens would time five tokens of work
against a hundred and twenty-eight and report the difference as speed.

**And the second finding, which nobody was looking for.** With the pin on, time
is linear in length, and the line has two halves:

| model | fixed cost per request | cost per token | rate from the slope |
|---|---|---|---|
| Qwen3-0.6B-Q8_0 | 987.1 ms | 13.141 ms | **76.1 tok/s** |
| SmolLM2-135M-Q4_0 | 141.5 ms | 1.930 ms | **518.2 tok/s** |

The obvious way to report a rate — total time divided by tokens — gives this
instead:

| length | Qwen3 | SmolLM2 |
|---|---|---|
| 8 | 7.3 tok/s | 50.9 tok/s |
| 16 | 13.4 | 93.0 |
| 32 | 22.7 | 157.5 |
| 64 | 35.2 | 241.3 |
| 128 | 47.9 | 329.5 |

**A 6.6-fold swing in the "rate" of one model, from nothing but the length
somebody picked.** Neither end is its speed. Its speed is the slope, and the
rest is the intercept being amortized differently.

**At the length MCF benchmarks by default, 70% of the run is not generation at
all** — 987 of 1,407 ms for Qwen3, 141 of 203 ms for SmolLM2, both almost
exactly seven tenths. A thirty-two-token benchmark is mostly a measurement of
loading a model and reading a prompt.

**What a token of mismatch is worth**, which is what the operator's earlier
question about prompt tokenization needed: **0.93%–0.95% of a 32-token run per
token.** So the two-token spread of the standard question is worth about 1.9% —
below this machine's own repeat spread of **1.7%–9.5%, median 5.1%**, but
*systematic*: it pushes every repeat the same way and does not average out.

**What follows, and it changes what a benchmark is.**

1. **Pin the length, or the axis is fiction** for any model that stops early.
2. **A rate is a slope, not a quotient.** Reporting tokens per second from a
   single length reports the length. A rate needs at least two.
3. **Two claims, not one.** *This model generates at N tokens per second* is the
   slope. *This configuration answers a thirty-two-token reply in M
   milliseconds* is the total, and is a different, equally honest statement that
   must name its length. MCF reports something shaped like the first and
   measured like the second.
4. **Match the arms' token counts exactly.** One token is ~0.95%; the floor is
   ~5%; a systematic bias under the floor is still a bias.
5. **The floor bounds the question.** A comparison asked to resolve less than
   about 5% here is being asked something this machine cannot answer at this
   shape, and should say so rather than take more pairs.

**Conditions, because these are numbers and not opinions.** One machine, the
provisioned llama.cpp at a pinned commit, a fresh process per run with the model
loaded each time and no warm-up — which is the *cold* regime MCF's `--cold` uses
and the one where the intercept is largest. A daemon holding a model resident
would have a much smaller fixed cost and the same slope; that is the next thing
to measure, and it is not measured here. `prototypes/generation-timing` is the
instrument, and A20 keeps every figure above out of anything MCF publishes.

**Closed on the wire it happened on (B-396).** *At most n* and *exactly n* are
now two requests: a generation carries `pinned`, the engine is told to run past
its end of text, and the account's own count is read back by whoever divides
by it — the ladder keeps only a pair whose runs produced one and seventeen, and
`mcf bench` stops at the first trial whose count is not the pin. Against
Qwen3-VL-2B served by the daemon, the same chat turn asked for 64 tokens
produced **3** and stopped at the model's end of text unpinned, and **64**,
stopped at the limit, pinned. The completion tool honours `--ignore-eos` and
does not count, and its account says `exactly_but_uncounted` rather than
`exactly` (A21).

## 118 · F118 — Generation is not a constant-rate process: the rate halves with context depth, and the depth a machine can reach is bounded by memory rather than by the model (B-396, B-397, D19, §3.4, A19, F117)

**The operator's design, tested.** The proposal was to let every model generate
from the same near-empty prompt for a fixed time — fifteen seconds, thirty, a
minute, two, four — and count the tokens, on the reasoning that a long enough
run dilutes the per-request fixed cost, a one-token prompt cannot cost one
vocabulary more than another, and a suppressed stop token stops verbosity
leaking in. All three are right, and better than the length ladder
[F117](#117--f117--the-pinned-length-was-declared-and-never-enforced-and-a-rate-computed-the-obvious-way-is-a-property-of-the-length-you-chose-b-081-d19-a19-34-f116)
used.

The design was changed in one respect: a ladder of **token counts** rather than
of durations, because killing a generation at fifteen seconds loses whatever is
in the output buffer and turns an exact count into an estimate. The axis is
swapped and the information is the same, plus one thing the duration form cannot
give — the *marginal* rate between rungs, which is the rate at that depth.

**The result contradicts the hope that the ladder would be conclusive on its
own.** Marginal rate, three repeats a rung:

| model | 128→256 | 256→512 | 512→1024 | 1024→2048 | 2048→4096 | change | machine noise |
|---|---|---|---|---|---|---|---|
| Qwen3-0.6B-Q8_0 | 73.7 | 69.9 | 67.4 | 59.1 | **46.4** | **−37.1%** | 0.8% |
| SmolLM2-135M-Q4_0 | 516.9 | 475.5 | 414.6 | 320.8 | **234.9** | **−54.6%** | 2.8% |

Each token attends to every token before it, so per-token cost rises as the run
goes on. The decay is an order of magnitude larger than the machine's own
repeat-to-repeat noise.

**It is not thermal**, which is why the machine was watched: the processor die
moved 59.6 → 61.4 °C and 63.2 → 61.2 °C across the deepest runs. A rate that
fell because the machine got hot would be a condition rather than a property,
and this one is the property.

**And the headline rate is not even monotonic.** Tokens over the whole run —
what a fixed-duration test reports — for Qwen3: 46.9, 57.4, 63.0, **65.1**,
62.0, 53.0. It peaks near a thousand tokens, because amortizing the fixed cost
pushes it up while context decay pulls it down and the two cross. *Measure for
longer to get a better number* does not hold: there is a length that flatters
each model and it is a different length for each.

**Two consequences for comparison.** A fixed-duration comparison is **biased
against the faster model** — it reaches deeper context in the same time and pays
more per token there. And *speed* is not a scalar property of a model at all: it
is a function of depth, so *which model is faster* is ill-posed until a depth is
named.

**The operator then found the axis this measurement left uncontrolled.**
Context length depends on the model *and* on the space the model is allowed to
occupy. Checked rather than agreed with: the engine had taken each model's own
declared training context, so the two ladders above ran at **8,192 and 40,960**
allocated context — a fivefold difference in key/value cache, never set and
never recorded.

What that costs, measured on SmolLM2-135M: allocating 512 against 8,192 is
**187 MB against 359 MB** of resident memory. **172 MB of key/value cache,
nearly twice the size of the weights.**

So:

* the allocation is a condition and MCF's own condition floor has a
  `context_length` field that has read `unknown` in every run this project has
  ever taken;
* two arms at their respective defaults are two allocations, which is a
  difference nobody declared;
* and the part of the decay curve a person ever reaches is **bounded by their
  memory**, not by the model — a machine that can afford 512 tokens of context
  never operates where the rate has halved.

**What this does not invalidate.** The 4,096-token rung sits inside both
allocations, so no eviction or context-shift occurred and the decay above is
genuine attention growth rather than an artefact of hitting a limit. What it
does mean is that the *magnitudes* are conditioned on an allocation nobody
chose, and the prototype now sets it, holds it still across arms, and records it
— refusing to combine readings taken at two.

**Still unmeasured**, and named rather than assumed: whether the rate at a given
depth depends on the allocation itself; the warm regime, where a resident model
has a far smaller fixed cost and MCF actually serves; and the cost of a *prompt*
token, since both ladders varied only the generated length.

## 119 · F119 — The allocation effect was the running order, and the fall-off is the memory bus (B-400, F118, A12, A21, D19)

**Retracting a number I gave the operator.** The context ladder appeared to
show that a model runs slower at the same depth when the context is allocated
small — 16.6% for Qwen3-0.6B, monotone across five allocations. It walked the
allocations in ascending order and wrote no temperature beside a reading, so an
allocation effect and a drift over the session had the same shape in that file.

Interleaved, every allocation visited once per pass:

| model | spread across allocations | across passes |
|---|---|---|
| Qwen3-0.6B | 2.5% | 2.2% |
| Qwen3-1.7B | 1.1% | 1.2% |
| SmolLM2-135M f16 | 2.3% | 3.1% |
| SmolLM2-135M IQ3_M | 5.0% | 3.9% |

The allocation does nothing the pass does not also do. What the ladder measured
was its own first readings: `c=1024` ran first, immediately after a server
start, behind an eight-token warm-up that was not enough. At depth 64 it read
15.469 ms/token there; interleaved it reads 12.878. A16's discipline about
stated conditions is not satisfied by stating *some* of them, and an ascending
sweep with no condition recorded per reading cannot separate its variable from
its order. A12 governs the outcome: where the earlier reading and the
controlled one disagree, the controlled one is right.

**What the fall-off actually is.** Each generated token makes attention re-read
the whole KV cache. From the headers: Qwen3-0.6B holds 112 KiB per token of
depth, so at depth 4096 it re-reads 470 MB for every single token; SmolLM2-135M
holds 22.5 KiB. Dividing the measured slope into that traffic gives a
bandwidth — 43–48 GB/s for Qwen3, stable from depth 192 to 24,575.

Measured on this machine: 55.8 GB/s from DRAM at sixteen threads, 77.5 GB/s
in-cache single-threaded, with both cache edges visible where the part says
they are (a 36% fall between 24 MB and 32 MB on one thread — one core
complex's L3; a 53% fall between 64 MB and 96 MB on sixteen — both). So Qwen3
is reading its cache at DRAM speed and is bound by the bus. SmolLM2, whose
cache stays inside L3 over most of the range anyone measures, is not.

The two models' curves are produced by different mechanisms, which is why one
extrapolates and the other does not — and the discriminator is arithmetic on
the header, available before anything runs.

## 120 · F120 — A prefilled depth costs what a generated one costs and arrives twenty times sooner, which retires the long path rather than speeding it up (B-400, F119, A18, A11)

**The long path was buying depth, not accuracy.** To measure what a token
costs at depth 4096 the cache must hold 4096 tokens. Every timing prototype
here reached that depth by generating them, which is quadratic and spends
almost all of its time on tokens nobody is timing. A prompt reaches the same
depth by prefill, and prefill is batched.

Whether the two are interchangeable is an empirical question — the cache holds
the same number of entries either way, but not the same entries:

| model | depth 576 | 1088 | 2112 |
|---|---|---|---|
| Qwen3-0.6B | −1.8% | −2.1% | −1.2% |
| SmolLM2-135M | −0.7% | +0.2% | +2.8% |

Every difference is inside the repeat-to-repeat noise, and prefill reached
those depths **10–24× faster**. Five depths on a small model now take about six
seconds.

**The comparison had to be made properly to say that.** The generated readings
are band midpoints and land nowhere near the prefilled depths; matching each
prefilled depth against the nearest band compares two different depths and
manufactured a disagreement of up to 21%, which is what the first version of
this analysis reported. Interpolating the generated curve at the prefilled
depth is the comparison that answers the question asked.

**Why this retires the long path rather than shortening it.** A depth cheap to
reach is a depth that can be *measured*, and the estimator laboratory was
unambiguous that a fitted curve cannot state its own error: a two-sigma
interval, which claims to hold the truth 95% of the time, covered it between 4%
and 79%; and where the truth bends past the fitted range every candidate form
is about 41% wrong at every fit depth, with nothing in the shallow readings to
announce it. Fitting a flexible form is worse still — a quadratic fitted below
depth 512 and asked about 12,288 is wrong by 1,230% at the median.

A18 separates a test from a benchmark; this separates a long measurement from a
necessary one. [long-tests.md](long-tests.md) carries the resulting discipline
and audits every long test here: three prototypes retire, seven tiers are
irreducible because duration, concurrency, the network or a second build *is*
the variable they vary.

## 121 · F121 — The fall-off predicted from the file header on all 27 models, and the one outlier was an architecture the arithmetic did not describe (B-400, F120, A7, A21, B16)

**Every model here, measured, against its own header.** Twenty-seven models
probed at five depths each, the slope fitted past the L3 crossing, and compared
with KV bytes per token of depth divided by this machine's measured bandwidth.
Scaling each group by a single constant:

| regime | constant | error | n |
|---|---|---|---|
| bandwidth-bound | 0.83 | −5% to +7% | 6 |
| latency-bound | 0.56 | −6% to +72% | 16 |

The split is decided from the header before anything runs: a bandwidth argument
needs each layer's read to be large enough to stream, and 2 KiB is where these
models divide. Above it — the Qwen3 family from 0.6B to 8B across four
quantizations — the header predicts the slope to within 7%. Below it the read
is bound by latency instead, the constant is looser, and the slope is measured
rather than predicted. Measuring costs 15–30 seconds.

**Quantization does not move the slope.** Eleven SmolLM2 builds from Q2_K to
f16 give 0.000726 to 0.000790 — a 9% spread across a fourfold range of weight
sizes. That is the mechanism confirming itself: the fall-off is the KV cache
being re-read, and the cache is f16 whatever the weights are. What quantization
moves is the intercept.

**The one outlier was right and the model was wrong.** gemma-3-270m missed the
prediction by 3.2–5.7×, always flatter. Its header says
`gemma3.attention.sliding_window = 512`: most of its layers never attend beyond
a 512-token window, so their cost stops growing with depth and they contribute
nothing to the slope. Only the full-attention layers do — one in every six.
Counting those brings it to 0.54–0.95, inside the band every other
latency-bound model occupies.

The interleave is not in the header; the engine hard-codes it. So it is named
per architecture rather than guessed, and an architecture with a sliding window
and no entry is reported as unpredictable rather than predicted wrongly (A7).
One outlier in twenty-seven found a whole class of architecture the arithmetic
did not describe, which is the argument for running the corpus rather than two
convenient models.

## 122 · F122 — The predictor generalised by being made to refuse: two architectures agree to 8%, and every model it cannot describe now says so (B-400, F121, A2, A7, A9, A21)

**The question the operator asked.** The corpus that produced
[F121](#121--f121--the-fall-off-predicted-from-the-file-header-on-all-27-models-and-the-one-outlier-was-an-architecture-the-arithmetic-did-not-describe-b-400-f120-a7-a21-b16)
held four architectures, and every model whose slope the arithmetic predicted
well was a Qwen3. A tool calibrated on one family that prints a number for
anything is not a measuring tool. Three things were wrong and all three were
fixable.

**Five models had produced nothing, and nothing said so.** The sweep allocated
8192 for every model and asked for depths up to 6144. Llama-160M is trained to
2048, stories15M to 128, all-MiniLM to 512: the prefill exceeded their context,
the engine answered HTTP 400, and `|| true` swallowed it. The table looked
complete because the missing rows were missing.

Now each model plans its depths from its own declared context, a refusal is
written as a row carrying the engine's own words, and a model that cannot be
probed at all still produces a row saying why. all-MiniLM refuses with *"the
current context does not logits computation"* — it is an embedding model and
was never going to generate. That is a result (A9), not an absence.

**The arithmetic now declines what it cannot describe.** It assumed every
architecture keeps a K-and-V cache that grows with depth. A state-space or
recurrent model keeps a fixed-size state, so its fall-off is flat and a
predicted slope would be confidently wrong rather than imprecise; multi-head
latent attention stores a compressed latent, so layers × heads × (key + value)
is the wrong product. `describe()` returns one of four verdicts — described,
no-growing-cache, not-described, header-incomplete — and only the first carries
a number.

Ten constructed headers exercise it, for architectures not on this machine,
because the property under test is not that the predictor is right about
everything but that it is **never confidently wrong**. It also predicts
correctly for multi-query attention and for the common case where head length
is implied by the embedding width rather than stated. MCF's own acquisition
refused a mamba repository on the same grounds while this was being written:
the configuration states no blocks or KV heads, and MCF will not guess a shape.

**And a second architecture was acquired and measured.** Phi-3-mini is `phi3`,
reads 12 KiB per layer against Qwen3's 4 KiB, and holds 384 KiB per token of
depth — three and a half times Qwen3's traffic:

| regime | constant | error | n | architectures |
|---|---|---|---|---|
| bandwidth-bound | 0.86 | −8% to +5% | 7 | qwen3, phi3 |
| latency-bound | 0.56 | −22% to +72% | 17 | llama, gemma3, qwen2 |

Phi-3 lands at 0.91, inside the band Qwen3 occupies, across 0.6B to 8B and five
quantizations. That is the claim generalising once, which is not the same as
generalising — two architectures is better than one and is still two.

The latency-bound constant did the opposite: Qwen2-0.5B came in at 0.43 and
widened the range to −22%/+72%. That regime is not predictable from the header
and must be measured, which costs 15–40 seconds. The honest split is that the
prediction is offered where it has been shown to hold and the measurement is
taken everywhere else — and which of the two applies is decided from the header
before anything runs.

## 123 · F123 — Counted rather than inferred: a token reads the whole model and the whole cache, so both halves of the curve are in the file (B-400, F121, F122, A11, A21, D19)

**The operator installed perf, and the mechanism could be counted instead of
argued from slopes.**

The counter had to be calibrated first, and the obvious one was wrong. The AMD
UMC controller PMU exposes `event` and `rdwrmask` and no named events; a
read-only and a write-only workload of known size gave counts that did not
separate by mask, and `rdwrmask=0x3` returned zero. Guessing at an undocumented
encoding is not measurement. `l3_lookup_state.l3_miss` is documented, one miss
is one cache line, and it calibrates: reading a known 512 MB to 4,096 MB gives
a slope of **58.0, 64.5 and 63.4 bytes per miss** against the 64 a line
actually is. That is an instrument.

**The experiment.** Two generations at the same depth differing only in tokens
produced — 32 against 160 — so the difference cancels prefill, model load and
everything else that happens once, leaving decode traffic alone. Qwen3-0.6B at
depth 4096, three passes:

| | bytes |
|---|---|
| counted, 128 extra tokens | **153.9 GB** |
| the KV cache alone | 60.1 GB — 2.56× short |
| the weights, re-read per token (639 MB file) | 81.8 GB |
| weights + cache | **142.0 GB — ratio 1.08** |

A generated token reads the whole model *and* the whole cache. The slope
argument had accounted for only one of them.

**So the intercept was never a mystery either.** It had been treated as the
number that must be measured because only the slope followed from the header.
It follows too — it is the weights divided by bandwidth:

    ms/token(depth) = (active weight bytes + depth x growing KV bytes) / bandwidth

Against every model measured here:

| | intercept ratio | slope ratio | n |
|---|---|---|---|
| bandwidth-bound | 0.82–1.03, median 0.95 | 0.79–0.91, median 0.86 | 7 |
| latency-bound | 0.78–0.96, median 0.89 | 0.36–0.71, median 0.56 | 17 |

The intercept generalises where the slope does not, and for a reason: reading
the weights is one long sequential stream whatever the architecture, while the
cache read is strided and small for models with few KV heads, which is the
latency-bound case.

**One outlier, and it was an architecture again.** TinyMixtral's intercept was
over-predicted by 1.79×. It is a mixture of four experts using two per token,
so most of the file is never touched — and 4/2 is 2. Scaling by
`expert_used_count / expert_count` brings it to 0.90, inside the band. That
assumes the experts dominate the file, which is stated rather than hidden, and
a dense model has no expert count and is scaled by one.

This is the third time an outlier has turned out to be a real architectural
feature written in the header that the arithmetic did not read — the sliding
window in [F121](#121--f121--the-fall-off-predicted-from-the-file-header-on-all-27-models-and-the-one-outlier-was-an-architecture-the-arithmetic-did-not-describe-b-400-f120-a7-a21-b16),
the growing-cache assumption in
[F122](#122--f122--the-predictor-generalised-by-being-made-to-refuse-two-architectures-agree-to-8-and-every-model-it-cannot-describe-now-says-so-b-400-f121-a2-a7-a9-a21),
and the expert count here.

**What perf is not.** It is not a dependency. It was used once, on this
machine, to check that the bytes moved are the bytes the arithmetic claims.
Nothing shipped needs it: kernel-coupled, privileged, absent in containers, no
Windows equivalent. The bandwidth constant the curve depends on is measured by
a userspace benchmark that needs no privilege on any platform, and the cache
topology it needs is world-readable — `index3: size=32768K shared_with=0-7,16-23`
states the 32 MiB per core complex that a bandwidth cliff had been used to
infer.

## 124 · F124 — The whole curve from the file, on seven architectures, with a state-space model as the control that has no curve at all (B-400, F123, A6, A7, A12, A21)

**The operator asked for an accurate measurement rather than a number
calibrated on one family.** Five architectures were acquired — mamba, rwkv6,
llama-3.2, gemma2, starcoder2 — bringing the corpus to seven, and the model
was refitted against all of them. Four things were wrong with it and each was
found by a model that did not fit.

**The negative control has no curve, and that is the point.** A state-space
model keeps a fixed state rather than a growing cache, so its cost per token
should not rise with depth at all. Measured across depths 576 to 3648:

| model | rise, shallowest to deepest |
|---|---|
| mamba-130m | **−0.1%** |
| rwkv-6-world-1.6b | **+1.0%** |
| gemma-3-270m | +29.8% |
| gemma-2-2b | +36.5% |
| starcoder2-3b | +64.0% |

Both were refused by name as `no-growing-cache` from their real headers before
being run, and neither declares a context length — correctly, since there is no
cache to bound. The refusal that
[F122](#122--f122--the-predictor-generalised-by-being-made-to-refuse-two-architectures-agree-to-8-and-every-model-it-cannot-describe-now-says-so-b-400-f121-a2-a7-a9-a21)
tested against a constructed header now has a measurement behind it.

**A sliding window is piecewise in depth, not a property of the model.** Below
the window a windowed layer has not filled it and grows exactly like a full one.
gemma-2's window is 4096 and every depth first measured was under it, so its
slope matched all 26 layers growing, ratio 1.01 — the window was invisible.
Probed past it: 0.002297 below against 0.001234 past, a factor of 0.54. Half its
layers keep growing, so one in two attends to the whole context, **derived from
measurement rather than cited**.

**The binary "streams or does not" split was the wrong shape.** It put
Llama-3.2-1B on the wrong side of a step at exactly 2048 B. Achieved bandwidth
is a curve in how much each layer reads contiguously, and a second term was
needed: starcoder2 shares 2 KV heads across 24 query heads and was 62% out on
read size alone. Grouped-query attention reads a cache entry once and uses it
for several heads, so a high ratio means more arithmetic per byte:

    achieved = 0.984 x read/(read + 433) / (1 + 0.0668 (query per KV head - 1))

Median residual 4.8%, worst 17.8%, over 27 models and seven architectures. Both
constants are this machine's and are refitted per machine.

**And the weights do not pay the cache's penalty.** They are one long
sequential stream whatever the architecture. Applying the cache's achieved
fraction to the intercept made it worse — median ratio 1.49 against 0.90
without. Two reads, two efficiencies.

The whole curve, from the file, with nothing generated:

    ms/token(d) = active weights / (BW x 0.90)
                + d x growing KV bytes / (BW x achieved(read, sharing))

| | n | median | 90th | worst |
|---|---|---|---|---|
| weights exceed 2× L3 | 96 | **3.6%** | 8.4% | **14.7%** |
| weights fit near L3 — **declined** | 44 | 1.8% | 11.1% | 62.9% |

The second row is the limit and it is the same cache boundary again. A model
whose weights fit in L3 is re-read from cache, not memory, and an intercept at
DRAM speed over-predicts it — Llama-160M holds about 100 MB against a 64 MiB L3
and was out by a factor of two. The model is not wrong there; it is being
applied where it does not hold, and it now says so instead.

## 125 · F125 — Five ways to state a speed, scored against each other: one probe and the header beats measuring everything, and beats the file alone (B-400, F124, A6, A18, A20)

**The operator asked whether the curve is reliable or the approach needs
rethinking.** Both, as it turns out: the curve holds up, and it is not the best
use of it.

**First, the accuracy claim was in-sample.** F124's 3.6% was measured against
the same readings its constants were fitted to. Holding out one architecture
entirely and refitting without it:

| | median | 90th | worst |
|---|---|---|---|
| in-sample | 3.5% | — | — |
| **held out** | **4.2%** | 7.9% | 13.7% |

The gap is small, which is the reassuring answer — the model is not memorising
architectures. But it is the held-out number that a user's unseen model faces,
and it is the one to quote.

**Then every approach, scored on depths it was not given.** Cost is per model,
taken from the readings themselves.

| approach | median | 90th | worst | cost | needs |
|---|---|---|---|---|---|
| scalar rate | 14.2% | 45.6% | 65.2% | 3s | one generation |
| predicted | 4.2% | 8.6% | 14.3% | **0s** | the file only |
| **anchored** | **2.2%** | **5.0%** | 11.8% | **3s** | one probe + header |
| two point | 0.8% | 2.9% | 4.6% | 11s | two probes |
| every point | 0.4% | 1.8% | 3.5% | 26s | a probe per depth |

The top row is what MCF does today and what nearly every published
tokens-per-second figure is. It is wrong by 14% at the median and 65% at worst,
for the same three seconds that buys 2.2%.

**And past the measured range the ordering changes.** Two probes carry
measurement noise in their slope, and extrapolation amplifies it; the header's
slope carries none. Over every extrapolated point:

| | median | 90th | worst |
|---|---|---|---|
| two probes | 2.7% | 11.0% | 19.7% |
| **anchored** | 2.7% | **5.1%** | **11.8%** |
| predicted | 4.3% | 9.4% | 14.3% |

Same median as two probes, **half the tail, a third of the cost**, and at 8×
beyond the probe it is 2.5% where two probes are 7.6%.

**It also dissolves F124's declined band.** Weights that fit in L3 are re-read
from cache, which broke the computed intercept — Llama-160M by a factor of two.
That failure was only ever in the intercept; a growing cache leaves L3 whatever
the weights do. Measuring the intercept absorbs the residency without modelling
it:

| | computed | anchored |
|---|---|---|
| weights reach DRAM | 3.5% median, 13.3% worst | **1.5%**, 11.4% |
| weights near L3 — *was declined* | 1.5%, 9.1% | **1.4%, 3.7%** |

There is no longer a band of models the tool has to refuse.

**What the physics was actually for.** Not the numbers — the anchored method
takes its intercept from a measurement and beats the fully computed curve. What
the arithmetic supplies is the *shape*: that cost is a straight line in depth,
because attention re-reads a cache growing linearly. The synthetic laboratory
fitted an unknown form and was ~41% wrong past its range with no way to know.
Knowing it is a line is what lets one probe reach eight times beyond itself.
The physics bought the shape, and the shape is what extrapolates.

## 126 · F126 — A probe's length is not a free knob: short probes read optimistically, and with that fixed every size to 70B fits five minutes (B-400, F125, A6, A20, D19)

**The operator asked whether 26 seconds was only because the models were
small, and offered a five-minute budget for very accurate numbers.** It was:
26s was a mean over mostly small models. Measured per model, the full sweep is
7 seconds for SmolLM2-135M and 174 for Qwen3-8B.

**One assumption had to be tested before any projection could be believed.**
Every cost estimate rests on how many tokens a probe times, and 128 was a
number chosen when the first probe was written, not measured. Cutting it to 32
would have quartered every large-model estimate. Measured at depth 2048,
against a long reference:

| tokens timed | Qwen3-1.7B | Qwen3-8B |
|---|---|---|
| 8 | **−14.2%** | — |
| 16 | −6.2% | — |
| 32 | −3.7% | **−2.6%** |
| 64 | −2.2% | −2.0% |
| 128 | −0.6% | −0.6% |
| 256 | reference | +0.0% |

**A short probe is biased, not merely noisy**, and biased in the flattering
direction: the tokens immediately after a prefill still hit a warm cache, so
they run fast and the model is reported quicker than it is. SmolLM2-135M shows
none of it — its cache never left L3 to begin with, which is the same boundary
that has explained every other exception here.

The bias does not worsen with model size: −0.6% at 128 tokens on both a 1.7B
and an 8B. So 128 is right, and 32 would have cost 2.6–3.7% against a method
whose entire error budget is 2.2%. The knob was not free and the cheaper
projections were wrong.

**Prefill scales more gently than decode**, measured across 27 models: rate
= 662 × GB^−0.77 tokens per second. Not −1.0, because prefill is batched and
does not re-read the weights per token. For a large model at a deep probe the
*prefill* is the expensive half, not the tokens being timed — which is why a
plan that insists on the deepest depth throws itself away when a shallower
second probe would have fitted.

**What five minutes buys, by size:**

| model | plan | probes | deepest | takes | median | worst |
|---|---|---|---|---|---|---|
| 0.6B–8B | every depth | 6 | 16384 | 56s–4.3m | **0.4%** | 3.5% |
| 14B | both ends | 2 | 16384 | 3.1m | 0.8% | 4.6% |
| 32B | both ends | 2 | 8192 | 3.8m | 0.8% | 4.6% |
| 70B | both ends | 2 | 2048 | 4.5m | 0.8% | 4.6% |

Every size to 70B comes inside the budget at 0.8% median or better. At a
60-second budget the same planner degrades honestly instead of overrunning:
70B falls back to the computed curve at 4.2%, 32B to a single anchored probe at
2.2%, and the accuracy quoted changes with it.

Those accuracy figures are the held-out measurements of
[F125](#125--f125--five-ways-to-state-a-speed-scored-against-each-other-one-probe-and-the-header-beats-measuring-everything-and-beats-the-file-alone-b-400-f124-a6-a18-a20),
not estimates of what the plan might achieve.

## 127 · F127 — The sampling rules made machine-checkable, and the GPU that cannot be tested because the engine has no backend for it (B-400, F126, A2, A21, B16)

**Four rules, set by the operator, enforced rather than described.**
`lab_rules.py` puts every model on this machine through every device and every
budget — 576 plans — and fails by rule number. Two defects were found by writing
them down.

**The planner was breaking rule 2 already.** It appended `context − 256` as a
final depth so that a windowed model would be probed past its window, which
produced depths of 7936 and 16128 — sitting in readings this repository
already holds. The cap is now taken by halving: the deepest sampled depth is
half the window, which leaves room for the probe's own generated tokens without
ever leaving the powers of two.

**Nothing had ever asked whether the window fitted in memory.** The ceiling had
been the model's trained context alone. It is now the smaller of that and what
the device's free memory holds — weights, cache at that depth, and the engine's
buffers, with headroom. On this machine Qwen3-8B is capped at 32768 by its
trained context on the CPU, and the 2.1 GB card cannot hold its weights at all
and is told so.

**And rule 3 surfaced something no measurement here had.** It asks for CPU and
GPU both. This machine has an RTX 5080 with 16.3 GB and CUDA 13.1 installed —
and MCF provisions llama.cpp in a Fedora container carrying neither, with
`GGML_NATIVE=OFF` and no `GGML_CUDA`. The binary answers `--list-devices` with
`(none)`.

So **every timing in this repository is a CPU timing**, and the GPU arm cannot
be run until a CUDA-enabled build is provisioned. That was true before this rule
existed; what is new is that the plan says so instead of silently reporting the
CPU as though it were the machine (A21: what the engine can do is not what the
hardware has).

**The estimate is a range, because it was scored.** Rule 4 wants a time before
the test starts. The point estimate is unbiased — median ratio 0.96 over 136
measured probes — but it runs 0.58× at the 5th percentile and 1.42× at the 95th,
because prefill throughput varies with quantization in a way neither model size
nor attention geometry predicts. A two-term physical fit, separating the
per-token matmul from the O(depth²) attention, was no better and worse in the
tail.

So the quote is a range and **the budget is tested against the slow end**: a
plan that promised five minutes may not take six. Under a 300-second cap
Qwen3-8B takes *both ends* at 85–209s rather than *every depth*, whose 259s
point estimate could reach 447s. The accuracy quoted falls from 0.4% to 0.8%
along with it, so the cheaper plan cannot borrow the fuller one's precision.

## 128 · F128 — A CUDA build provisioned, the first GPU timings taken, and the CPU's constants do not transfer to it (B-400, F127, A12, A21, F31)

**Rule 3 asks for CPU and GPU both, and MCF could not do it.** The provisioned
llama.cpp answered `--list-devices` with `(none)`, so every timing in this
repository was a CPU timing. A second component now builds the same commit with
a CUDA backend, and getting there found three defects.

**A semicolon in a configure flag was a shell command.** `CMAKE_CUDA_ARCHITECTURES`
takes a semicolon-separated list, and the recipe pasted the flags into a bash
script unquoted. Bash ended the cmake line at the semicolon and tried to run
`120` as a command: exit 127, after a *clean* configure that had silently
dropped the architecture flag. Every configure argument is now shell-quoted, so
the next flag with a metacharacter in it does not find the same hole.

**The recipe assumed one package manager.** It said `dnf` and `rpm` outright,
which was true of the only image there was. The CUDA toolkit ships on Ubuntu,
and the second component failed on `dnf: command not found` before compiling
anything. Packaging is now declared per component.

**And F31's lesson had to be learned twice.** The first working build linked the
container's `libcudart.so.12` and would not start on this host, which carries
CUDA 13 — the same shape as the shared-library failure that made
`BUILD_SHARED_LIBS=OFF` necessary. `CMAKE_CUDA_RUNTIME_LIBRARY=Static` was not
enough on its own; cuBLAS and NCCL were still dynamic, and it took
`GGML_STATIC=ON` and `GGML_CUDA_NCCL=OFF` before the binary ran where it landed.

**The first GPU measurements.** RTX 5080 against this machine's CPU, same
models, same depths:

| model | CPU ms/token | GPU ms/token | speedup |
|---|---|---|---|
| SmolLM2-135M | 2.30–4.86 | 0.91–1.01 | **2.5–4.8×** |
| Qwen3-0.6B | 13.9–27.1 | 1.56–2.54 | 8.9–10.7× |
| gemma-2-2b | 35.0–43.3 | 3.26–3.66 | 10.7–11.8× |
| Phi-3-mini | 47.8–59.7 | 4.03–4.76 | 11.9–12.6× |
| Qwen3-8B | 89.0–107.9 | 6.43–7.66 | **13.8–14.5×** |

Median 11.8×, and the speedup rises with model size — which is the bandwidth
story restating itself, since a larger model is more thoroughly bandwidth-bound
and the card has an order of magnitude more of it.

**The constants do not transfer, and that is the finding.** Substituting a GPU
bandwidth into the same formula:

| | median ratio | range |
|---|---|---|
| intercept | 1.00 | 0.17–1.35 |
| slope | **1.70** | 1.29–2.44 |

The slope is wrong by seventy per cent everywhere, and the bandwidth implied by
the intercepts (642 GB/s) and by the slopes (1095 GB/s) disagree by a factor of
1.7 about a card specified near 960. `achieved_fraction` was fitted against a
CPU's cache hierarchy and describes nothing about a GPU's.

SmolLM2-135M is the sharpest case at 0.17: its 92 MB of weights take almost no
time to read at GPU bandwidth, so its intercept is not a bandwidth term at all
but a floor — about 0.9 ms of per-token launch overhead that the CPU model has
no term for.

So the constants are **per device**, not per machine, and a GPU needs its own
fit with a term the CPU form does not have. What survives the crossing is the
*shape* — a straight line in depth, from a cache re-read once per token — which
is what has been carrying the method all along.

## 129 · F129 — A second surface arrived and the tripwire watching for one did not fire, because it was written against four guesses at its name (B-401, B-072, A22, B16)

**MCF now has a terminal application**, and building it set off three checks
and failed to set off a fourth — which is the finding.

**The one that did not fire.** B-072 left a tripwire for the day a second
surface appeared: an assertion that no crate existed under any of
`mcf-window`, `mcf-web`, `mcf-ui` or `mcf-client`, so that whoever added one
would be made to enumerate its actions. The surface arrived as **`mcf-tui`**
and the check passed. Four guesses, none of them right.

This is the sixth time in this project a guard has covered the shape of the
place it was written for rather than the property it was written about — after
F103, F105, F106, F112 and F113 — and the correction is the same one every
time. A surface is now recognised by *what it does*: a crate that reaches
`control::Request` is a client of the control plane, whatever it is called.
Both halves are verified by breaking them — an undeclared crate that reaches
the wire fails by name, and an action naming a request the control plane lacks
fails by name.

**The three that did fire, and were right to.**

* `no_tier_inherits_a_daemon` required the new module be written down, so no
  test tier can drive it and inherit a running daemon (F104).
* `reference_model_neutrality` refused a test that used a real model's filename
  as an example. B28 keeps MCF neutral about which model is *the* reference, and
  a name in a test is a name in the tree.
* `workspace_shape` refused the crate until its dependency edges were declared.
  It also refused an edge that was not needed: the checks read the action table
  as source text, so linking the crate was a dependency that bought nothing.

**What the application is.** Zero dependencies, on both platforms. Raw input is
`tcgetattr` where there is a termios and `SetConsoleMode` where there is a
console, both declared by hand — the `unsafe_code` opt-in the workspace
anticipated, taken per module with the reason at the site, which is exactly what
the lint's own comment asks for.

It costs nothing while nobody types: the terminal is put into a mode where a
read blocks until a key arrives, so the loop is a thread asleep in `read` rather
than a loop that spins. There is no timer and no redraw not caused by a key,
which is B-071's requirement met by having nothing to do rather than by doing it
efficiently.

The terminal is given back on three paths and not one — the guard on every
ordinary return, a panic hook for the path `Drop` does not run on, and an
explicit call before returning. A terminal left raw is a terminal that stops
echoing what the operator types, and they will not know why.

**And it refuses where there is no terminal**, rather than drawing at nothing:
piped output is a fact about where MCF was pointed, not a fault, so the refusal
names the two commands that answer the same questions with no display attached.

## 153 · F153 — The console's buttons could not be reached: Tab sits below the printable range, the arm that named it was dead, and the tests handed the screen a key the decoder never produced (B-404, B-401, A22, F130, F131)

Wiring the console's buttons to the daemon (B-404) ended with a driver that
runs `mcf tui` in a pseudo-terminal and types at it — Right, Enter, Tab,
Down, Enter — because every unit test of the screen calls `act` with a
`Key` already made, and the question was whether a person's keyboard
produces that key. It does not. The console had drawn *Quick Run*, *Run
Selected* and *Back* since B-404's first cut and nothing typed at it had
ever reached them.

**Tab is 0x09, and the decoder's printable arm starts at 0x20.** The
`Character('\t')` arm in the console's key handling was written on the
assumption that a tab arrives as a character, the way a letter does; the
decoder that turns bytes into keys hands back `Character` only for
`0x20..=0x7e`, and lets the byte fall through otherwise. So the arm was
dead code from the day it was written — reachable by no byte sequence a
terminal sends — and the compiler has no way of knowing that one match
arm's pattern is a value another function never constructs.

**The tests could not see it** because they were testing the wrong seam.
Every test of the buttons — moving between them, pressing them, refusing
a press while a run is going — fed `Key::Character('\t')` straight into
`act`, and `act` did what it said. The decoder had its own tests, and
none of them asked about a tab, because nothing said a tab mattered. The
seam that failed was between two things each tested alone, which is the
shape of F130 (a discovery that matched a name) and F131 (an SDL constant
named the wrong event): a layer's tests pass, and the layer above assumed
a fact about it that was never asserted anywhere.

**Found by driving the thing.** The pseudo-terminal driver captured the
frame after each key, and the frame after Tab was the frame before it —
the cursor still on a row, the buttons still plain. That is the only kind
of test that would have found this: one that starts from bytes, because
the assumption being tested was about bytes. The driver is not a shipped
test (it needs a daemon, a model and minutes), but the byte is now: the
decoder's tests assert that `\t` decodes, and to what.

**The fix is a variant, not a character.** `Key::Tab` is decoded from
0x09 the way Enter is from 0x0d and Escape from 0x1b — the control keys
are their own variants and the printable range stays what it is. The
console's arm names `Key::Tab`; the tests feed it the key the decoder
makes. A headless surface is only equal to the window (A22) when its
keys can be pressed.

**What the same driver found once Tab worked**, each of them a seam
between two things tested alone. The console's Quick Run sent half the
model's window as the deepest rung — 131,072 on a 2B model whose button
said *28 s – 68 s* — where the window's sends 1,024, the depth that
estimate was for; the daemon's own estimate of 298–730 s under a button
promising a minute was the first frame to say so. One depth and one
estimate now serve both (B-072). A finished run was heard again on every
key pressed: the console kept it so the screen could still say what it
was, and `hear` took it again each pass — filling the rows again with a
longer time, *ran 30 s*, *32 s*, *39 s*, and asking the daemon for the
status and the listing again each time. And the console left the moment
the ladder finished: it polls while a run is going and waits for a key
otherwise, and decided what a read of nothing meant *after* the read,
by asking again whether it was busy — so the pass in which the run
finished took its own timed-out poll for the end of input. Which of the
two a read was is now decided once, before it. None of these could be
seen from a test that hands the console a key, because each is about
what happens between two keys, or between a key and the daemon.

**And one thing the driver could not find, written down instead.** The
daemon answers one connection at a time, so a `stop` sent while the
ladder ran sat unanswered for the ladder's length, and the console's
status word stood at *Idle* through a run it could not ask about. The
console now says what it is running from its own job; the daemon
answering while it measures is B-425.

## 152 · F152 — No rung deeper than 2,048 was ever measured: the served engine was reused by model alone, refused every turn longer than its first window, and the refusal was written down as a pair that did not separate (B-424, A2, A7, A9, F133)

Filling the *Memory ceiling* row (B-424) meant reading the engine's peak
resident memory after each rung, and the first ladder run to look at it —
Qwen3-VL-2B, to 8,192 — measured 512, 1,024 and 2,048 and said of 4,096
and 8,192 *not measured: no pair of runs at this depth separated: the
longer one finished no later than the shorter, so their difference is not
a cost*. Both rungs took under a second. Nothing had run.

**The server was reused by model alone.** A served engine is started with
a window sized to the request — what was sent plus what was asked for,
doubled, with a floor of 4,096 — and held between requests so that the
next turn on the same model does not load it again. The rule for keeping
it was *same model*. A ladder's first rung opened a 4,096-token window;
every rung after it was sent to that server; and llama-server refuses a
turn longer than its window with a 400, which the adapter turned into a
failure with the engine's words in it, which `timed_generation` turned into
`None`, which `one_depth` could not tell from a pair of timings in the
wrong order — the one other way a rung comes back with nothing. So the
reason written down was the wrong one: not *the engine refused this
depth*, but *the runs did not separate* (A2, A7).

**And it had always been so.** The reuse rule predates the ladder going
past 2,048, and no `ModelTimed` entry in this machine's record — 1,633
entries — has ever carried a measured rung at 4,096 or deeper. Every
*deepest* the console and the window accepted above 2,048 was a promise
the daemon could not keep, and the record said *not measured* for each of
them with a sentence that named a cause that had not happened. A rung that
is not measured is a result (A9); a result with the wrong reason is worse
than none, because it stops the reader looking.

**Two fixes, both small.** A server whose window is smaller than the turn
needs is not that turn's server: `through_served` restarts it when
`held.window < window`, and the window each rung ran in is a condition of
its account (`conditions.window`) and of its reading. And a rung that
produced nothing says why in the engine's words — `timed_generation` hands
back the failure's detail and category, and `one_depth` prefers a refusal
over the non-separating sentence when it has one, so that the next such
rung reads *the engine produced nothing at this depth: …* with the 400 in
it. The same ladder after the fix, on the same model and the same
processor build: 512 at 15.4 ms a token, 1,024 at 17.3, 2,048 at 19.4,
4,096 at 66.4 and 8,192 at 139.1 — every rung measured, and the last two
in windows of 8,226 and 16,418 that the record now names. The memory row
this was found on reads off the same run: 117,346 bytes a token of window
between windows of 4,096 and 16,418, against 114,688 planned from the
header — 2.3 % over, which is the engine's own bookkeeping around the
cache — and 3.64 GB held at 8,192 deep.

**What it says about the check that was missing.** The window is a
condition of every timing (§3.4) and was recorded on the request that
opened the server, then never compared with the request that reused it.
A condition that is not compared on reuse is a declaration, not an
observation (A21); the comparison is now the reuse rule.

## 151 · F151 — A latent cache was withheld by the report and sized at nearly twice by placement, from one header read two ways; now one reading serves both (B-038, B-072, A7, F150)

The second file read for F150 — the latent-attention model with a dense
first block among its expert blocks — showed two surfaces of MCF disagreeing
about its key/value cache. `mcf explain` said *not sized: this model caches
a compressed latent rather than its keys and values, and its width is not
the key width the header names*. Six lines further down, placement said
*131072 tokens of context fit there*, a figure resolved from a cache
`mcf-serve` had sized without hesitation as key/value heads × (key length +
value length) × block count × 2 bytes. Two readings of one header, in two
crates, one refusing and one confident — and the confident one wrong
(B-072).

**What the header names is exactly the latent.** The converter for a
latent-attention model writes the latent's width plus its rotary part as
`attention.key_length` (576 here: a rank of 512 and 64 of rope), the
latent alone as `attention.value_length` (512), and one key/value head. The
engine's cache allocator, read at the pinned commit rather than remembered
(`llama-kv-cache.cpp`: `has_v = !is_mla`), makes a key cache of that width
per block and **no value cache at all** — the values are read back out of
the same latent. So the cache is 1 × 576 × 47 × 2 = 54,144 bytes per
token; the serving path had 1 × (576 + 512) × 47 × 2 = 102,272, 1.9× the
truth, and the report had nothing. The report's refusal was written when
the widths under those keys were assumed to be a head's, and its stated
reason — *not the key width the header names* — was the reverse of the
case.

**One reading.** `mcf_serve::engines::shape_of` now builds the hub's
`Shape` from the same `anatomy::work` the report shows — the blocks that
attend (F150), the key/value heads, and *what one head keeps per position*:
a key and a value, or one latent that serves as both. The hub's `Shape`
carried a `head_dimension` and doubled it in its own arithmetic, which is
the assumption *a value for every key* written as a formula; it carries
`per_head` now, and a configuration's `head_dim` is doubled where it is
read, where the assumption is true of what is being read. The report says
what it multiplied: *1 head(s) keeping one latent of 576 per position,
which is read back as both key and value — no value cache*. And the
key/value heads row, which had said *not read from the directory* for want
of a key projection, reads the one head off the latent projection
`attn_kv_a_mqa`, whose width is the key length.

**What it cost here, and what it would have.** On this machine the
placement line reads *131072 tokens fit* before and after, because the
window MCF samples at is the largest power of two under the declared
context, and 10 GiB of cache and 19 GiB of cache both leave room for that
one. On a machine with less free — or for the declared 202,752 tokens,
which the corrected cache holds in 10.2 GiB — the overstated figure would
have refused a context the model fits, by a number the report beside it
said could not be computed. Each of those was a hidden choice (§3.15), and
none is now.

## 150 · F150 — The cache of a hybrid was sized by the header's block count and came out four times too large; the same file's heads read as thirty-two against sixteen declared, and its BF16 tensors as a type MCF does not read (B-038, A7, A21, F16)

`mcf explain` was run over a file whose architecture keeps a recurrent
state in three of every four blocks and attends in the fourth — forty-eight
blocks, twelve of them holding keys and values. Three things in the report
were wrong, and each was the header believed where the directory could have
been read (A21).

**The cache was sized by the block count.** The key/value cache per token
was *layers × key/value heads × (key + value widths) × 2 bytes* with
*layers* the header's `block_count`, so the file's 24,576 bytes per token
were stated as 98,304 and the cache at its declared context as 24 GiB
rather than 6. The arithmetic in `mcf-serve` — the one placement is
resolved from and *largest context* is computed from — carried the same
count, with a comment saying a hybrid's cache is overstated here and that
overstating errs toward refusing. It does: four times the cache is a
context that fits refused (F16 made the same choice for a configuration
that does not say which layers attend). A GGUF header does not say either;
its tensor directory does. Every block that attends holds an `attn_k` or
`attn_qkv` projection, every block that keeps a recurrent state holds
`ssm_*` tensors, and counting the blocks by what each holds gives the
number of blocks that cache. Both surfaces now size the cache from that
count — one census, read in `mcf-standin`, used by the explain report and
the serving arithmetic alike (B-072) — and a header read with no directory
behind it falls back to the block count, overstating rather than
understating. The report says which it did: *across the 12 of 48 blocks
that keep keys — the other 36 keep a fixed recurrent state*.

**The blocks were a count, and they are not alike.** The same census is a
section of the report: each shape of block, which blocks are that shape,
what the shape is made of (attention or a recurrent state; a dense
feed-forward, experts with or without a shared one, or neither), how many
parameters the shape holds and at what encoding — a range where the blocks
of one shape are encoded differently, as the first block of this file is.
A second file read this way showed a dense first block among forty-six
expert blocks, which its header says nowhere.

**Heads were read off the wrong projection.** Observed attention heads were
the query projection's width divided by the key width. The recurrent
architecture's attention blocks carry a gate beside every query in the same
tensor, so the projection is twice the heads' width, and the row said
*declared 16, found 32, DISAGREE* about a file that was fine — a
disagreement MCF invented, on a line whose whole purpose is to be believed
over the header. The output projection gathers exactly one head's output
per head, and its first dimension divided by the head's value width — the
latent value width where the header declares one, else the value width,
else the key width — reads sixteen on this file and twenty on the latent-
attention file whose query projection does not exist under that name at
all. The query projection is the fallback, where there is no output
projection to read.

**Five tensors were a type MCF does not read.** GGML type 30 is `BF16`,
which a converter leaves the tensors it keeps at training precision in.
Unknown, it left five tensors unsized, which left the file's weight bytes
*not totalled*, the attention and feed-forward parts *unsized*, and every
recurrent block's bits *unsized* — a hole of 28 million elements in a total
of 79 billion, correctly refused as a total (A7) and cheaply closed. And the
recurrent tensors themselves counted as *other*, with the decay `ssm_a` —
a name with no `.weight` after it — read as a tensor whose leaf was `0`.
They are a part now, *recurrent state*, and a name's last segment is a
suffix only when it is one.

[`blocks`]: ../crates/mcf-standin/src/anatomy/blocks.rs

## 149 · F149 — The daemon found its engines once and never looked again, so an engine built while it ran was a prefix on disk and *no engine* on the socket (F31, A7, B-367, B-072)

The window was asked to build the engine a model needs when the model is
held, and the first design question was where the build should run. It ran
in `mcf-cli` — `mcf provision` spawned podman itself — so the window's only
route was to spawn the command, which is a second implementation of *what a
build does* on the day one surface diverges from the other (B-072). The
builder moved into `mcf-serve` as a library with two callers, and the daemon
gained a `provision` request that streams the build's lines down the socket.

**What the move found.** The daemon sampled its engines once, at start-up —
deliberately, because asking a build what devices it has means running it,
and a status request that starts processes costs something (§3.13). The
consequence nobody had written down: a component provisioned while the
daemon ran — by `mcf provision` in another terminal, and now by the daemon's
own hand — was a complete prefix on disk and *no engine is installed yet* on
the socket until somebody restarted it. F31 was a build that succeeded and
could not run; this is a build that could run and the one process that
needed to know had stopped looking. Reporting *no engine* over a directory
holding one is A7 by the letter.

**What was done.** The engine list is held under a lock and looked for again
in one place: after a build this daemon finished. That is the one moment the
set is known to have changed and the one that needs no polling; a build from
the command line while a daemon runs is still invisible to it, and said so
here rather than fixed with a directory watch nobody asked for. The final
line of a build now carries `usable_engine`, which is whether the daemon
reaches what it built — so a build that completes and is not an engine is
said in the same line that says it completed.

**Exercised once for real.** A daemon started over an empty `provisioned`
directory with one model refused its settings with `needs_component:
llama.cpp` in the context; a `provision` request with no name chose that
component, said so in its first line, streamed 437 lines of the toolchain
install, the clone, the configure and the compile, and ended with the
prefix, the log, the record entry and `usable_engine: true`. The same
daemon, not restarted, then recommended settings for the model on the engine
it had built.

**What was not there to stream.** The first run of the same exercise sent
two lines: the choice and the outcome, with a fifty-second silence between
them, because the script sent cmake's output to files in the prefix and
nothing to the pipe the daemon was reading. A window that heard nothing for
the length of a build could not have told it from a hang (A2). The script
now announces each stage and copies the configure and build output through
`tee`, so the logs in the prefix are whole and the surface sees the build
move; `pipefail` was already set, so a failed cmake behind the tee still
fails the script.

**The command line goes through the daemon that is up.** The "still
invisible" above was true for one commit. `mcf provision` now connects to
the socket first; where a daemon answers and the build is bound for its own
root, it sends the same `provision` request the window sends, prints each
streamed line on the error stream and reports the final line in the wording
the local path uses — plus whose process built it and whether the daemon now
reaches it as an engine. Where nothing listens, or `--into` names a root the
daemon does not look under, the command builds in its own process and says
so in the report. One builder, one wording, two callers, and the CLI can do
what the window does (A22, B-072). Exercised once for real: a daemon started
over an empty `provisioned` directory said *it cannot run a model*; `mcf
provision` beside it streamed 437 lines and ended with *built by the daemon
/ the daemon now reaches it as an engine*; `mcf status` on the same
unrestarted daemon no longer listed anything it cannot do, and a second `mcf
provision` was told the prefix was already there.

[`provisioning`]: ../crates/mcf-serve/src/provisioning.rs

## 148 · F148 — Six readers told the operator MCF did not say why, over a body that said exactly why (A2, B-072)

`mcf settings <model>` on a model named the short way printed *refused — MCF
did not say why*. The daemon had said why, at length: `config.invalid`, *a
client sent something MCF cannot read*, wanted *a model this machine is
holding*, found `artifact.missing … this model could not be measured`. The
console read a key named `what` from the body; the encoding writes no such
key, and never has. The fallback text was a false statement about the daemon.

**Six copies.** `settings`, `host`, `measure`, `acquire`, the window's job
runner and its settings reader had each written the same three lines, and
`prompt`, `status` and `stop` printed the raw JSON line instead — a body a
person can decode but should not have to. None of them went through a reader,
because there was none: [`encode::failure`] had a writer and no counterpart.
B-072 is the rule that surfaces agree; here they agreed on being wrong.

**What was done.** One decoder, [`decode::failure_said`], that reads the
detail, the context and the cause chain back out as lines. Every client site
goes through it, and a check refuses a client source that carries the old
fallback or reads `what` from an answer body, so the seventh reader cannot
make the same mistake.

**What it does not fix.** The refusal is still worded from the daemon's side
— *a client sent something MCF cannot read* is what the generic refusal
constructor says about every request it turns down, and the useful sentence
sits one line down under *wanted*. That is a wording question for
`control::refused` and is left where it is; the defect here was that none of
it reached the person.

[`encode::failure`]: ../crates/mcf-record/src/encode.rs
[`decode::failure_said`]: ../crates/mcf-record/src/decode.rs

## 147 · F147 — One real prompt found five defects in prompt analysis, and the report presented a run that separated nothing exactly as it presents one that works (§3.15, A6, A7, §3.4)

The prompt was *write a class in c#. the class handles all the basic math
functions. each function should output extremely accurate results.* — three
sentences, an ordinary request. Everything below came out of running it once.

**The floor came out at 94.3%.** Removing a sentence carrying no instruction
moved almost the whole answer, so the column separated nothing: two of the
three sentences scored *below* the floor and the third was two points above
it. The report drew three confident bars and put the number that invalidates
them underneath, in the same typeface and the same voice as everything else. A
reader reads the bars. A measurement that failed and a measurement that worked
looked identical, which is the failure — the same screen had shown a floor of
0.0% on a well-specified prompt an hour earlier.

**Answers were cut at 160 tokens and nothing said they were.** Every
generation stopped inside the import block, so what the ablation compared was
six lines of preamble that shift wholesale when anything before them changes.
That is a measurement of the preamble, and it is most of why the floor was so
high. The limit is now 600 and travels in the report: a figure computed from a
cut answer is a figure about a prefix, and that is a condition of it (§3.4,
A6).

**Settledness could not have measured anything.** Both engine paths set
temperature 0, so decoding is greedy and the seed cannot change the answer.
Asked at seeds 0, 1, 2 and 99, one prompt gave one answer four times. The
section could only ever print *this prompt settles the answer* — for every
prompt, on every model, forever. It was measuring the sampler. It now says so.
Measuring the real thing needs sampling, which changes every other figure in
the report, so it is named as unmeasured rather than quietly switched on.

**The splitter cut sentences out of numbers and identifiers.** `.` ended a
clause wherever it appeared, so `accurate to 0.001 tolerance` became *accurate
to 0.* and *001 tolerance.*, `arr.Length` became two, `System.Numerics` two. A
prompt about precision — which is what this one was about — was ablated on
fragments that were never sentences. A full stop now ends a sentence only
where whitespace follows it.

`e.g.` still splits, and that is held in a test rather than left to be found
again: telling an abbreviation from a sentence ending in the letter g needs a
list that is a fact about a language rather than about this prompt, and
guessing wrong the other way would silently join two real sentences. The
failure that remains splits one clause in two; the one removed split a number
in half.

**And the cost forecast added to the window a commit earlier was short by
one**, having forgotten the control generation: three sentences were forecast
at six and cost seven. Written a day before, by the same hand, while reading
the same file.

**What the sequence says about the feature.** Every one of these was visible
from a single run of an ordinary prompt, and none of them was visible from the
tests, which use fixtures whose answers are short and whose sentences are
clean. The report was built and checked on prompts that suit it. A tool that
tells you about your prompt has to be pointed at somebody else's.


## 146 · F146 — The completion tool was never given a window, so every generation opened the model's whole trained context: 81.3 GB resident to produce a few hundred tokens (F133, A6, A12, §3.15)

**F145 ended with something still unexplained**, and this is it. Runs kept
dying at their memory limit within seconds while `llama-server`, measured on
its own, sat comfortably inside it. Tracing every process in the group twice a
second found the answer in one line: the process was not `llama-server`.

    cur=68.1G anon=67.9G llama-completio:81.3G

**`llama-completion` at 81.3 GB resident**, for a 14.5 GB model, to generate a
few hundred tokens. MCF starts it once per generation and never passed it a
window at all, and llama.cpp reads an absent `--ctx-size` as *the model's whole
trained context* — 131,072 tokens, whose cache is most of that figure.

**This is F133, in the argument list beside where F133 was fixed.** The comment
in `served.rs` describes the identical defect for the server, at length: *this
was the literal `0`, which llama.cpp reads as the model's whole trained
context … the engine then allocated a cache for a window nobody asked for:
42 GiB resident for a 17.6 GiB model.* The server was given the window MCF
resolved. The completion tool, twenty lines away in `adapters.rs`, was given
nothing, and stayed that way.

**Why it survived being fixed next door.** Nothing measures what a request
costs. The window is invisible in the account, the tool is spawned and reaped
per request so no `mcf status` shows it, and the answers were correct
throughout — the model produced good text while holding eighty gigabytes to do
it. It only became visible when a memory limit turned an invisible cost into a
dead process, and even then it hid behind two other real defects (F144, F145)
that had to be fixed first before this one was the last one left.

**Sized to the request now**, in bytes rather than tokens: sizing it properly
would mean tokenizing the prompt, which needs the vocabulary, which needs the
model this command exists to hand to a subprocess. A token is several bytes so
counting bytes over-counts, which is the safe direction, and it is still four
orders of magnitude below what was being opened.

Measured on the same eval, same model, same cap: the group's charged memory
falls from **68.1 GB to 8.5 GB**, and the tool's resident size from 81.3 GB to
22.0 GB.

**The sequence is the lesson.** Three findings deep, each one real, each one
fixed, and the symptom unchanged until the last. The first two were found by
reasoning about where memory goes; this one was found by watching, once a
second, which process actually had it. When an explanation has been wrong
twice, the next step is not a third explanation.


## 145 · F145 — `mcf probe` held the whole model in the console beside the engine already holding it: 24.7 GB for a 14.5 GB file, and the memory nothing could account for (B-372, A22, F140)

**F144 ended with something unexplained**, and this is it. Runs were dying at
their memory limit in about thirteen seconds while the engine, measured on its
own, needed about 31 GB of a 40 GB allowance. Something took the rest and that
finding declined to guess what.

**It was the console.** Sampling every process in the group, largest first,
found `mcf probe` itself at **24.7 GB resident** — not the daemon, not
`llama-server`. `probe` opened with `std::fs::read(&path)`: the whole model
file, into the client's own memory, while the engine it was about to ask held
the same weights a few processes away. Two copies of one model, and only one
of them doing any work.

**Nothing needed it.** Every probe below that read — chat template, embedding,
language cost, structured output, thinking, tools, vision — uses those bytes
for `gguf::parse` and nothing else: the header, the vocabulary, what the file
declares. Not one reads a tensor. The bounded prefix `mcf run`, `mcf bench`
and `mcf explain` already use answers all of them.

Measured after: the same seven verdicts, and the whole group — daemon, probe
and engine together — peaks at **13.7 GB** where the probe alone had been
24.7 and the group had been dying at 40.

**This is F140 again, one command over**, and the two were written six days
apart. There the module doc said an explanation that quietly loaded four
gigabytes would be a surprise, above a line that loaded seventy-five. Here the
console loaded a model to ask a daemon about it. The shape is the same:
`std::fs::read` is the obvious way to get a file's bytes, it is correct, and
it is wrong at this size — and the repository already had the bounded reader
in three other places each time.

**Two more of the same, found by looking rather than by crashing.** `mcf embed`
reads the whole file and needs it — the stand-in runs on those bytes — but
`addressed_as`, in the daemon's own serving path, read the entire model to
parse a header and build a vocabulary. Every generation of a model somebody
had given an addressing paid for a full read of it. That one is fixed here
too; the embedding path genuinely wants the tensors and is left alone.

**What would have caught it.** `a_model_is_weighed_before_it_is_held` sweeps
for anything handing bytes to the *loader* without weighing them first, which
is the allocation F136 was about. Reading a file is not loading a model, so
neither `explain` nor `probe` was in its scope, and both read whole models
anyway. The class is *reading a model-sized file into memory*, and it is
wider than the class the check holds.


## 144 · F144 — MCF planned against the machine while running inside a limit, and its fixed overhead constant is thirteen times too small (A21, A19, B-372)

Two defects, found together, one fixed.

### The machine MCF was reading was not the one it was running in

`/proc/meminfo`'s `MemAvailable` describes the host. Under a container, or a
systemd scope with a memory limit, it goes on describing the host — and MCF
planned a context window against 119 GiB while running under a 40 GiB limit.
The engine was ended by the kernel sixteen seconds in.

This is A21 with the machine as the declaration. The figure was read honestly
and correctly; what was wrong is that it describes something other than the
thing the decision is about. It is the same shape as F136's start-up sample
and F138's single-part size: a true number, from the wrong subject.

**cgroup v2 says what the limit is**, and a limit may sit on the process's own
group or on any ancestor, so the smallest headroom found is the one that
binds. `mcf_core::hardware` now reads both and reports the smaller, and the
daemon asks it rather than reading `/proc/meminfo` a second time, so the
console and the daemon cannot come to different numbers about one machine
(B-072). Verified: inside a 40 GiB scope MCF reports 39.9 GiB available and
125.0 GiB total, and inside an 8 GiB scope it refuses a 17.5 GB model with
both figures — while uncapped it is unchanged.

### And the fixed overhead in the estimate is thirteen times too small

Fixing that did not stop the kill, so the estimate itself was measured against
the engine — which A19 asks for and which had never been done. `llama-server`
was started on a 14.5 GB model at three window sizes and its resident size
sampled:

| window | MCF's estimate | measured |
|---|---|---|
| 4,096 | ~15.6 GB | **21.9 GB** |
| 32,768 | ~20.2 GB | **26.3 GB** |
| 65,536 | ~25.0 GB | **31.3 GB** |

**The per-token part is right.** The measured slope is 153 KB a token against
the 160 KB `cache_bytes_per_token` computes from the header — close enough
that the header arithmetic is sound and should be left alone.

**The constant is what is wrong.** Extrapolated to an empty window the engine
holds about 21.3 GB for a 14.5 GB model, so what is neither weights nor cache
is close to **6.8 GB**. `OVERHEAD` is `512 << 20` — half a gigabyte, thirteen
times smaller — and its comment says it was *measured rather than guessed: a
0.6B model at a 512-token window held 187 MB resident against 359 MB at 8,192*.
That measurement was honest and was taken on a model twenty times smaller
than the ones MCF is now asked about. A constant derived from a 0.6B model was
carried forward to a 24B one.

An earlier draft of this finding put the gap at fifteen gigabytes, inferred
from a scope that died at 40 GB rather than from the engine. The engine says
6.3 GB at 65,536 tokens, consistently, and the inference was wrong — which is
the reason A19 asks for the independent value rather than the plausible one.

**What this means for every verdict that rests on it.** `fits`, the largest
window `resolve` will propose, the refusal that names two numbers, the figure
now shown beside the context setting in the window — all of them are computed
from this estimate. They are not wrong about their own arithmetic; they are
answering *what does MCF think this costs* when the reader believes they
answer *what will this cost*. A19 asks that a measurement be taken against an
independent value, and this one never has been: the estimate has never been
compared to what the engine actually took.

**Fixed after all, once there was enough to measure against.** Leaving it
alone was the wrong call: with the constant short, MCF sizes every window to
fill the memory it believes it has, so the engine was killed at a 40 GB
allowance and again at 64 — raising the allowance only bought a larger window
and the same death. There is no cap at which an underestimate stops mattering.

A fourth reading settled the shape. Overhead is 6.75, 6.56, 6.31 and 5.80 GB
at 4,096 / 32,768 / 65,536 / 131,072 tokens on the 14.5 GB model, and 7.79 GB
on the 17.5 GB one: it barely moves with the window and it tracks the weights.
`overhead_for` is now half the weights, floored at the old constant —
conservative on all three models including the 0.6B reading the original was
taken from, and predicting 42.75 GB where the engine at 131,072 tokens
measures 41.3.

Three models fitted to a straight line is not a law, and B-384 is still where
the relationship gets measured properly rather than from what was to hand. Recorded now because these figures are already on the
screen — beside the context setting, in the refusals that name two numbers —
and a figure a reader trusts is worse than no figure when nothing has checked
it.

**What is still not explained.** The scopes died at exactly their limits, in
about thirteen seconds, at 40 GB and again at 64 GB — while the measurements
above say the server needs about 31 GB at the window MCF chose. Something in
the run allocates more than the server does, and this finding does not say
what. It is written down unexplained rather than attributed to the nearest
plausible cause, which is what produced the fifteen-gigabyte figure above.


## 143 · F143 — Two surfaces counted one store and got ten and eleven: the daemon sent the distinction and the console dropped it (B-422, B-072)

B-422 settled that a projector belongs to a model rather than being one, and
`mcf list` says so: *10 model(s) … and 1 companion file(s) that belong to one
rather than being one*. `mcf status`, reading the same store through the
daemon, said *holding 11 model file(s)*.

**The daemon had always sent the distinction.** Every entry in the holding
answer carries a `companion` flag; the console asked for the list, counted its
length and ignored the field. So the two counts came from one source that
already knew the difference.

Neither line is false on its own — eleven files are held — and that is what
makes it the shape B-072 is about. A reader who saw both would have to work
out which of two numbers described what they had, with nothing on either
screen acknowledging the other. The fix is four lines and the value was
already on the wire.

**Found while measuring something else**, which is the usual way: the two
sentences appeared in one terminal a few lines apart while timing how long the
daemon takes to answer.

**The structural fact underneath is not fixed, and should be recorded rather
than quietly left.** `mcf list` never asks the daemon. It walks the store
itself, whether or not one is running — which is why it answers in 3 ms while
`mcf status`, which does ask, waits behind whatever the daemon is doing. The
daemon's own `runs` carries the note that *a console that opened the model
store itself would be a surface reaching past the wire, and two surfaces
reading the same file could disagree about it*. They did, and this finding is
that disagreement.

It is not a simple defect, because `mcf list` has to work with no daemon up,
and the obvious repair — ask the daemon where there is one — makes the most
used listing command wait behind generations that have nothing to do with it.
That trade is DEC-012's, not this finding's: which surface is authoritative
cannot be settled while only one request can be served at a time. What is
settled here is that when they both answer, they answer the same way.


## 142 · F142 — The engine's own `[end of text]` was recorded as the model's words, and MCF reported it did not know why generation stopped while holding the thing that said (A19, A21, A2, A7)

**llama.cpp's completion tool writes `[end of text]` on its own standard
output when the model emits its end-of-turn token** — in the same stream as
the answer, with nothing to separate them. MCF read that stream and passed all
of it on as what the model said.

**So a model that wrote `return result` had `return result [end of text]`
recorded against it.** In `mcf run` the operator sees words the model did not
write. In `mcf eval` the consequence is worse and quieter: the answer is
Python, and ` [end of text]` makes it a syntax error, so every unfenced answer
failed to run. A21 in its plainest form — a statement by the tool, presented
as an observation of the model.

**Found by disagreement between two models, not by reading the code.** A
2B general model scored on ten of twelve tasks; an 8B *coding* model came back
`unknown` on nine, including `merge-sorted`, which the 2B passed. A coding
model that cannot write merge sort is not the likely reading, and the raw
answer was correct Python with the marker welded to its last line. B-110's
laboratory measured MCF's output handling and reported it as the model's
ability.

**A7 kept it visible.** `unknown` is not zero: MCF refused to score answers
that would not run, and said so in the sentence that named the possibility.
Had the laboratory scored those nine as failures, the number would have looked
plausible — a smaller model doing worse — and nothing would have pointed at
the cause. The rule that stops MCF reporting ignorance as absence is what made
the defect legible.

**The same marker was the answer to the question beside it.** The account read
`stopped: unknown_the_engine_did_not_say`, under a comment explaining that
this engine prints text and exits and why it ended is not on the wire. It is
on the wire; it is the marker. A7 asks MCF not to claim what it does not know
and does not ask it to discard what it does — the same confusion as F138's
`placement: Unknown`, in a different subsystem, in the same week. The account
now reads `stop_token`, which is the word the served path already used.

**And the first fix was wrong in a way worth keeping.** Streaming means the
marker can arrive split across reads, so the obvious guard is to hold back any
suffix of the buffer that is a prefix of the marker. That is sufficient only
if the marker is last. The tool writes `[end of text]` and *then* two blank
lines, so by the end of the read the marker sat in the middle of the buffer,
the suffix under test was `\n\n`, and the whole thing went straight out — the
guard passing its own test while the symptom was unchanged. A fixed tail is
held back instead, which needs no reasoning about what follows what.


## 140 · F140 — `mcf explain` read the whole model into memory to look at its header: 16.41 GB to describe a file, beneath a note in the same module saying it must not (§3.11, A2, B-372)

**The module's own doc says it.** *Nothing here runs the model. Reading a
file's own description is cheap and safe; loading its weights is neither, and
an explanation that quietly loaded four gigabytes would be a surprise where
§3.11 asks for a decision.* Thirty lines below, the command began with
`std::fs::read(&path)` — the whole file, into memory, to read a header that
lives in its first few megabytes.

**Measured, on a model held here.** `mcf explain` on a 17.5 GB model peaked at
16.41 GB resident. On the 62 GB first part of a larger one it would take that
much again. On this machine, with a model already hosted, describing a model
could end the process describing it — or something else.

**Three uses, none of which needed the file.** The header, which
`gguf::parse` reads from a bounded prefix and which this module was *already*
reading that way in `language_cost` twenty lines further down. The size, which
the filesystem knows. And the sha256, which `mcf_core::integrity::digest_of`
already streams in fixed blocks, because `mcf check` needs exactly that.
Everything required was written, tested and in use elsewhere in the same
binary.

**16.41 GB to 0.13 GB, with no change in wall time.** The read is the same
number of bytes off the disk; what changed is that they are no longer all held
at once. The section still says *verified here, now*, and it is still true:
what stopped being required is room to hold a model in order to describe one.

### The test that came with it

Fixing F138 made a latent flake reachable, and the two mistakes in fixing
*that* are the more useful record.

**Sizing a model by the whole set moved the large ones onto the boundary.**
`the_placement_shown_is_the_placement_resolved` resolves a model twice — once
through the page, once directly — and asserts the two agree. A model whose
weights sit near what the machine can hold resolves differently depending on
how much memory was free at each moment, so under a workspace run, with
several test binaries competing, the two calls straddled the boundary. The
test failed on a real machine doing real work rather than on a defect.

**The first fix picked the wrong model.** *Smallest file* chose the 10.9 MB
first part of the 111.92 GB four-part model: the largest thing held here,
wearing the smallest file's size. F138 catching the test the same way it
caught the console, one commit later.

**The second was the real one.** The refusal names how much memory was free,
so comparing the page's text to a second call's text compared two live
measurements taken moments apart. The property is that *a reason travels with
the refusal*, not that the same bytes appear twice. An assertion that embeds a
measurement in its expected value is testing the machine's idleness.


## 139 · F139 — Eight commands MCF answers were absent from its own help, and six of them denied existing when typed: the console sent operators to two of them by name (A22, §3.15, A2, F135)

**A22 says the headless path can do everything the window can. It could — and
there was no way to find out.** `mcf host`, `mcf hosted`, `mcf unhost`,
`mcf settings`, `mcf measure`, `mcf share`, `mcf offered` and `mcf acquire`
were all parsed, dispatched and working. None appeared in `mcf --help`. The
only route to them was reading the source.

**Two of them MCF told operators to run.** `mcf status` prints *hosting:
nothing — `mcf host <model>` holds one where a program can reach it*.
`mcf explain` prints *`mcf settings <model>` shows every setting it would run
under*. Both commands worked. Neither was listed.

**And typed as instructed, six of the eight denied existing.** Each wanted an
argument, so the bare form matched no pattern and fell through to the
catch-all: `mcf host` answered *no such command: host*. An operator following
MCF's own instruction was told by MCF that MCF has no such command. That is
not a discoverability failure, it is a false statement (A2), and the operator
has no reason to doubt it.

**A stale claim in the same page.** The help's closing prose read *What MCF
cannot do yet is serve a model or time one.* `mcf host` serves a model on a
port; `mcf bench` and `mcf measure` time one. F135 is the finding about
exactly this — a status is prose, and prose does not fail a build — and it had
recurred in the most-read paragraph MCF has.

**A test was holding the defect in place.** `an_unknown_command_is_named_and_carries_its_input`
demonstrated A2 using `measure` as its example of an unknown command, which it
was when the test was written. `mcf measure` was implemented afterwards. The
test went on passing, now asserting that a command MCF *has* is reported as
unknown — the defect wearing the costume of the property. Its example is now a
word nobody will implement, and it checks that assumption before relying on
it.

**What holds it.** `the_help_is_the_surface` reads the console's argument
patterns for the commands they match and the usage text for the commands it
names, and requires the two lists to agree in both directions — a command
absent from the help is a capability only the source reveals, and a command in
the help that nothing answers is a fabricated one. A third test requires every
command taking an argument to have a bare form, so that none of them can deny
itself again.

**Both of its own false negatives are worth keeping in mind**, because they
are what source-scanning checks do wrong. It read `mcf lab` and `mcf recommend`
out of a *test* asserting those are not offered, and reported the help as
inventing them — so it now reads the usage block rather than the file. And it
read `["measure"]` out of the stale test above and concluded `measure` had a
bare form when it had none — so it now reads only the patterns, not the tests.
A check that scans text is reading a place where things that look alike are
not alike, and it has to be told where to look.

**It came back a third way (B-426).** The bare form was guarded; the form
with *more* than the parser's arms took was not. `mcf settings foo --bogus`
and `mcf cross-check foo --json` matched no arm and fell to the catch-all,
which answered *no such command: settings* — the denial this finding closed,
one argument further along. And the first word after a command was read as
its name whatever it was: `mcf measure --help` measured a model called
`--help`, and started a daemon to do it. The parser now reads the usage
table itself — which commands there are, and which of them want a name before
any flag — so a command the help lists is never denied, a flag where a name
goes is refused with the name the table wanted, and `--help` asked of any
command is that command's line of the table. The table is the one place, and
the tests walk it rather than a list of their own.


## 138 · F138 — Four places sized a model and one of them counted the whole of it: a 111 GB model planned against as 10.9 MB, and a context recommended that would take the machine down (B-422, B-072, A21, F136)

**A split GGUF is published in parts and an engine pointed at the first one
loads all of them.** B-422 taught the store this — *the parts of a model are
the model* — so `mcf list` shows a four-part model as one entry carrying the
set's length. Its own note records why: ungathered, *`mcf explain` could not
size one*.

**Three other places went on sizing a model with `std::fs::metadata` on the
single file they were handed.** The daemon's `recommend`, which is what
hosting plans from. `explain`'s `resolved_here`, which is what the placement
row shows. And `explain`'s speed projection, where size stands in for how much
work a generation is. Each got the first part.

**On one model held here the first part is 10.9 MB of a 111 GB set.** Four
orders of magnitude. The placement row, asked about that model, answered *the
processor: 262144 tokens of context fit there* — computed from weights of
10.9 MB against 129 GB of free memory, and confidently wrong in the direction
that hurts. A reader acting on it asks for a context whose cache alone is
larger than the machine. This is the same failure mode as F136 from the other
end: there the arithmetic was skipped, here it was fed a number from a
different model than the one being asked about.

**It is B-072's disagreement in its plainest form.** Two surfaces, one model,
two sizes — 111,334,654,784 bytes from `list` and 10,946,624 from `explain` —
with nothing on either page to tell a reader which they were looking at, or
that there was another.

**`bytes_of_the_whole` is now the one answer**, beside `held` in the store
where the gathering already lives, and the three callers use it. `explain`
prints the set's total beneath the file's own when they differ, because the
file's length is still the honest answer to *what did MCF read and hash* and
the reader needs both.

**Two things surfaced while confirming it, both about saying why.**

- Corrected, the model was refused — and reported as `placement: Unknown`,
  because `resolved_here` ended in `.ok()` and threw the reason away. *No
  engine is provisioned*, *this header does not say how it is shaped* and
  *this model is larger than this machine* all arrived as one word. The last
  is a thing MCF knows precisely and can put two numbers to. A7 is about
  saying what is not known; it is not a licence to decline to say what is.
- The reason, once carried, read *needs about 111.92 GB of memory and the
  largest device here has 129.15 GB free* — and then refused. A reader does
  that subtraction, gets a positive number, and concludes MCF is broken. What
  actually decides it is that MCF plans to at most 85% of what a device has
  free, and that was the one term the sentence omitted. A refusal whose stated
  numbers do not entail it is worse than a bare refusal (§3.15).

**The habit worth taking from it.** Every one of these four call sites is two
lines of obviously-correct code. `metadata(path).len()` is not wrong; it is an
answer to *how big is this file*, in a place that was asking *how big is this
model*. The defect was invisible at each site and only visible across them —
which is what B-072 is for, and why the answer belongs in one function that
every surface calls rather than in four that agree by coincidence.


## 137 · F137 — A supervisor that killed the one process it could name, then waited on the ones it could not: the deadline fired at 200 ms and the number beside it said thirty seconds (A3, A27, A4, B37)

**The §7.19 prototype exists to confirm that D4's substrate makes A3 cheap:
*no failure of a managed thing may take down MCF itself*. Its own test for the
hang case had been failing, taking the full thirty seconds of the child it was
supposed to give up on after two hundred milliseconds.**

**The deadline was never the problem.** `wait_until` polls, notices the
deadline, kills the child and returns `PastDeadline` — correctly, on time. What
followed undid it. The child was `/bin/sh -c 'sleep 30'`, and a shell running
one command forks it rather than becoming it, so `Child` named the shell and
the sleep was a grandchild. Killing what the supervisor could name left the
grandchild running and holding the write end of the stdout pipe. The drain
thread's `read_to_string` cannot end until every writer closes, so the join
blocked for the child's full lifetime — and `waited` was read *after* the join.

**Three defects, and each one alone would have been enough.**

- **The manager waited on the managed.** A supervisor whose deadline is
  enforceable in one function and unenforceable in the next has no deadline.
  This is the exact failure A3 names, appearing inside the prototype whose job
  is to demonstrate A3 holds.
- **The report was false.** The deadline fired at 200 ms; `waited` said
  30 s, because it was measured after joining the drain rather than after the
  supervision it describes. B37 is careful that the interval be monotonic and
  says nothing about measuring the right interval, which is how a correct clock
  produced a wrong number.
- **A process was left behind.** The comment at the kill reads *A27's habit at
  the smallest scale: MCF does not leave behind a process it started* — sitting
  directly above a call that leaves one behind whenever the child forked. It
  described the intent and nothing held the code to it. The proof was
  incidental: a run of the new test with the fix reverted leaked a `sleep`
  that was still running minutes later, and the next run detected it.

**What holds each now.** The child is spawned into its own process group, so
the negative of its identifier reaches everything it started — the only handle
there is on a process's descendants, and the reason this site opts into
`unsafe` for `kill(2)`. `waited` is taken before anything is joined. Past the
deadline the drain is not joined at all: what it has collected is taken and the
thread is left to end when the pipe does, because A4's partial outcome has to
be readable *while* it is still partial. `nothing_the_child_started_is_left_running`
checks `/proc` for a survivor, and fails without the process group.

**The general shape.** Every one of these is a comment that was true about the
intent and false about the code, in a file whose prose is unusually careful.
Two of the three would have been caught by asking *what does this line wait
on?* rather than *what does this line do?* — and the third by reading the
comment beside the kill as a claim rather than a label.


## 136 · F136 — The console weighed a model before loading it and the daemon did not, so a machine with no room for one was told by the kernel, and not necessarily in this process (B-372, A2, A22, B-072)

**MCF's own engine dequantizes to `f32`.** A quantized file comfortable on
disk becomes several times its size in memory, which is why B-372 wrote
`fits_dequantized`: arithmetic on a file's tensor directory against what the
platform says is free, refusing with both numbers named. It works, it is
tested, and the console calls it before it loads anything.

**The daemon did not call it.** `mcf run` reaches the same stand-in engine by
two roads — in-process when nothing is listening, over the socket when
something is — and only one of them weighed the model. The same file was
refused by the console and loaded by the daemon, which is precisely the
disagreement A22 and B-072 exist to prevent, and the road without the guard
is the one that is taken whenever a daemon happens to be up.

**What the unguarded road did.** It read the whole file into memory, then
allocated several times that again. Asked for a 62 GB model on a machine with
96 GB free, it would reach for 335 GB. Nothing reported anything, because
there was nothing left to report with: the kernel picked a process and ended
it. Sometimes that was the daemon. Sometimes it was whatever else the operator
had open — an editor, a session, work that had no connection to MCF beyond
sharing a machine with it.

**That last part is why this is worse than an ordinary A2 violation.** A
silent failure normally costs the caller their answer. This one spends a
resource that is not MCF's to spend, and the bill is presented to whoever the
kernel chooses. The operator sees an unrelated program disappear and has no
reason at all to suspect the thing they asked to load a model.

**A second defect sat inside the first.** The load branch carried a comment
saying *the previous resident, if any, is released here* — and it was not.
The new model was assigned at the end of the load, so the outgoing one stayed
resident throughout it, and the peak was two models and a file on a path whose
whole purpose is to hold one model at a time. The comment described the
intended design; nothing held the code to it.

**The remedy is order, and it is the same order the console already uses.**
Everything decidable from the header is decided from the header, before
anything large is read and before anything held is let go: an architecture MCF
was never taught, then whether this machine can hold the model dequantized.
Both are answerable from a few megabytes. Only then is the resident released,
and only then is the file read. A refusal now costs nothing and keeps what was
already loaded.

**What holds it.** `a_model_is_weighed_before_it_is_held` sweeps the shipped
tree for anything that hands bytes to the loader and does not weigh them
first. It is a coarse instrument — presence, not dominance — and it caught a
second unguarded site in the same change, in the cross-check, which is exactly
the class of thing a coarse instrument is for. Sites that are deliberately
unweighed are declared with their reason, so the next one is a decision
somebody writes down rather than an omission nobody notices.


## 135 · F135 — One change made three register entries false, and none of them said so: a status is prose, and prose does not fail a build (B-036, B-038, B-040, A1)

**B-416 gave MCF a port, engine resolution and recorded settings. It also made
three statements in the register untrue, and the register kept making them.**

- B-036 said MCF was local *by construction rather than by configuration*:
  *there is no bind address, no port and no flag, so exposure is not something
  a mistake can do because it is not something MCF can do.* Hosting a model is
  being reachable. The port arrived and the sentence stayed.
- B-038 said `mcf explain` shows *what MCF would choose*. It showed *refused:
  more than one is provisioned* and *placement: the processor* for a model MCF
  resolves to a graphics card.
- B-040 said what remained was *a timed answer, and `serve` handing a model to
  a client rather than a command loading one per request*. Both had landed —
  in B-412 and B-416 — under other item numbers.

**None of these is a code defect and all three are the same failure.** The
register's totals are checked: `the_register_counts_itself` reads every row
and refuses a headline that disagrees with the table. What no check reads is
whether a *status* is still true, because a status is a paragraph of English
and there is nothing in it a machine can compare against the code.

**The first of the three was the dangerous one.** A stale safety claim is
worse than an absent one: somebody reads *exposure is not something MCF can
do*, believes it, and stops looking. The other two cost a reader's trust in
the screen; that one could cost somebody their assumption about a network.

**The remedy is not another check, it is an order of operations.** When a
change lands, the items it touched are re-read before the next one is started
— and where a claim narrowed, it is narrowed in writing and given a check that
holds the narrower version. B-036 now has one: what the loopback constant is,
that no setting carries an address, and that the control plane never grew a
port. That is the shape to aim for. A claim worth making in the register is
usually a claim worth checking, and the ones that cannot be checked are the
ones to re-read by hand.


## 134 · F134 — A type MCF already had, written a second time: sampling in thousandths, without the one distinction the original carries (B-419, A1, B-281)

**B-416 wanted every setting a hosted model runs under to be visible, so it
wrote a `Sampling` type**: temperature, top-p, top-k, a repetition penalty and
a seed, held in thousandths as whole numbers because a shipped crate holds no
floating point and a NaN one division from a record is how a measurement
starts lying. The reasoning was sound and every part of the answer already
existed.

`mcf_core::configuration::Sampling` holds the same five things. It holds them
in `Thousandths`, which is the same representation for the same reason, and
which renders as `{}.{:03}` — the same decimal the copy formatted by hand.
It has been there since the configuration model was written.

**And the original carries a distinction the copy did not.** Each of its
values is an `Attested`: declared, verified, or unknown. A temperature a
publisher wrote in a model's metadata and one a sweep measured are not the
same kind of fact, and B-281 is the item that turns on exactly that — a
recommended sampling renders as *declared* until a sweep promotes it. The copy
had plain numbers, so anything built on it would have had to invent that
distinction again or lose it.

**What it cost was nothing and what it nearly cost was the rule.** The copy was
never wired to anything: it shipped as dead code and was removed the moment
somebody looked. Had it been wired first, MCF would have had two answers to
*what sampling is this* and a check would eventually have found them
disagreeing — which is the shape of
[F129](#129--f129--a-second-surface-arrived-and-the-tripwire-watching-for-one-did-not-fire-because-it-was-written-against-four-guesses-at-its-name-b-401-b-072-a22-b16),
[F130](#130--f130--two-engines-were-provisioned-and-invisible-because-discovery-matched-a-name-and-sixty-four-sentences-a-person-reads-cite-a-document-they-have-never-seen-b-402-b-403-a21-a7-b16)
and F133 one more time, in a different material: a thing that was really there
and invisible to the person writing next to it.

**The remedy is the same as those, and this one has no check.** A name-match
can be caught mechanically (B-417); *somebody wrote a type that existed*
cannot, not usefully — the shapes are too varied and the false positives would
swamp it. What is left is the habit: search before writing, and treat a
representation that feels obviously right as evidence that somebody has
already had the thought.


## 133 · F133 — MCF said a model ran on the graphics card and ran it on the processor: the layer count was written into the source as zero, and it cost 4.9× (B-416, A6, A12, §3.15)

**The operator asked why there were no settings when choosing a model to host.
There were none, and one of the values nobody could see was wrong.** The
provisioned engine was started with its arguments written into the source:

    --ctx-size 0  -ngl 0

`-ngl 0` is *no layers on the graphics card*. So `engines::resolve` did its
arithmetic, chose the CUDA build and an RTX 5080, reported **runs on NVIDIA
GeForce RTX 5080** — and then MCF started an engine that put none of the model
there. Every generation MCF served through a provisioned engine, and every
figure the depth measurement produced, was taken on the processor under a
label that said otherwise.

**Measured, on the same model and the same build:**

| Layers on the card | Tokens a second |
|---|---|
| none | 158.4 |
| all | 770.5 |

Four point nine times. A6 asks that no number appear without its conditions;
this was worse than a missing condition, because the condition was *stated and
false*. A12 says reality wins over the simulator, and what MCF reported was
neither — it was a plan, printed as though it were what happened.

**A second one, one level up.** Fixing the flag was not enough: the daemon
found its engine by asking for *a* provisioned llama, which returned the
processor build while the settings said the CUDA one. So the first corrected
run still had 97 MiB of card memory in use and the CPU binary in `ps`. The
engine is now taken by name from the list the daemon probed at start, and the
card holds 1,167 MiB.

**The remedy is not a better default, it is a visible one.** §3.15: MCF doing
something other than the plain thing must never be invisible. Every setting a
hosted model runs under is now a field with a recommendation MCF computed —
context, layers, engine, device, threads, batch, flash attention, residency,
port, key — and both surfaces list them with what was advised beside anything
somebody moved. What was chosen and what was recommended are two facts, and
the record carries both so they can disagree in writing.

**And hosting means something now: the engine listens.** MCF does not
implement an inference API and does not claim to — it provisions an engine
that has one, starts it under settings that are written down, and supervises
it. What answers is `llama-server`'s own OpenAI-compatible interface on
`127.0.0.1`, and both surfaces say whose it is. Binding every interface would
put somebody's model on their network, which is a decision they make rather
than one MCF makes for them, so the loopback address is not a default but the
only address MCF binds — and the gate check now reads the constant rather than
trusting its name.

**Three smaller things the work turned up, each a guess where a check would
have done.**

The default port was 11434, chosen with a comment saying nothing common used
it. That is Ollama's default, and Ollama had it on this machine. The port
moved, but the lesson is the comment: the guess was written down as though it
were a finding. MCF now *asks* whether the port is free before starting
anything, because an engine that cannot bind exits with a status and no
sentence — and *the server stopped before it began answering* is a true report
of the wrong thing (A2).

The readiness probe was written with `writeln!`, which ended the request
`\r\n\n`. HTTP wants a blank line, so the server waited for a request that
never finished and MCF waited for an answer that never came: a health check
that **hangs** rather than fails, which is the worst shape a check can have.

And the sampling settings were `f32`. A shipped crate holds no floating point,
because a NaN one division from a record is how a measurement starts lying —
they are thousandths as whole numbers now, converted to a decimal once, on the
way to the engine's command line.


## 132 · F132 — Three of the window's new capabilities were caught by checks before they shipped: a progress bar that reported zero at the moment it finished, a plan that sampled hardware from the serving path, and a timing in floating point (B-412, A7, B4, A6)

**The window can now fetch a model, time one, and talk to one**, and each of
those is a control-plane request the command line also sends — `Offered`,
`Acquire`, `Measure`, and the `Generate` that was already there. What the
daemon does behind them is `mcf_hub`'s, which both surfaces now call, so *this
model will run here* cannot come out differently depending on which of MCF's
own surfaces asked.

**Three defects were caught by the check suite rather than by use, and each
was a rule this repository already had.**

*A progress bar that reported zero at the moment it finished.* Download
progress is read off the partial file's size, because a transfer that reported
its own progress would be making a claim §3.7 says not to take on trust. But
the partial file is *renamed* when the transfer completes, so the last reading
before completion found no file and reported `0`. A bar would have jumped back
to the beginning at the moment it filled. The fault is the `unwrap_or(0)` —
the same conflation of *absent* with *zero* that A7 exists to forbid, written
by the person enforcing it. It now keeps the furthest size actually seen, and
a file that is not there moves nothing.

*A plan that sampled hardware from the serving path.* Judging which published
variant fits needs to know free memory, and the function that did it called
`Machine::read()`. That was correct while only the command line called it, and
became wrong the moment the daemon did: B4 forbids MCF sampling hardware from
anywhere that runs unasked, because a daemon reading counters to answer a
question is one of the competitors it reports (§3.8). The remedy was not an
exemption — the figure is now a *parameter*, so the command line reads the
machine because somebody ran a command, and the daemon passes what it already
knew. The number used is in the answer either way, which A6 wanted anyway.

*A timing in floating point.* The measurement computed milliseconds per token
in `f64`. A shipped crate holds no float, because a NaN one division away from
a record is how a measurement starts lying (A6, A1). All of it is integer
nanoseconds now, formatted to milliseconds only where a person reads it —
which is also more honest, since a duration is a count of ticks.

**The measurement subtracts, and that is the whole of its method.** A single
timed generation at depth measures loading the model, reading the prompt, and
producing the tokens. Only the third is what *speed at depth* means. So each
depth is run twice, producing one token and seventeen, and the per-token cost
is the difference over sixteen; whatever loading and prefill cost, they are in
both. That matters more here than it usually would, because the daemon loads a
model per request and drops it (DEC-018) — unsubtracted, a shallow reading
would be mostly the loading. Measured on this machine, with three repeats a
depth and the median taken:

| Depth | Milliseconds a token |
|---|---|
| 512 | 8.012 |
| 1 024 | 8.243 |
| 2 048 | 9.107 |
| 4 096 | 10.236 |

**A third one, found while making measurements durable (B-414), and it is the
ninth instance of the same pattern.** Fixing the hosting path left the
*generation* path untouched, and that path found its engine through
`adapters::provisioned_llama`, which matched `component == "llama.cpp"`
exactly. `llama.cpp-cuda` was invisible to it. So every generation MCF served
and every depth reading it took went through the processor build — not because
of the layer count this time, but because the CUDA build was never a
candidate. That is
[F129](#129--f129--a-second-surface-arrived-and-the-tripwire-watching-for-one-did-not-fire-because-it-was-written-against-four-guesses-at-its-name-b-401-b-072-a22-b16)
and
[F130](#130--f130--two-engines-were-provisioned-and-invisible-because-discovery-matched-a-name-and-sixty-four-sentences-a-person-reads-cite-a-document-they-have-never-seen-b-402-b-403-a21-a7-b16)
again, in a copy of the discovery that was fixed in `engines.rs` and not here.

Matching by shape then made *two* engines visible, and the generation path
refused to choose between them — correctly, because choosing is not its
business. The daemon already resolves a model to an engine and a device, so it
now passes that answer down rather than letting the generation path ask the
question again. The measurement moved from 7.3–9.3 milliseconds a token to
1.3–1.5:

| Depth | Processor build | CUDA build |
|---|---|---|
| 512 | 7.337 | 1.529 |
| 1 024 | 7.641 | 1.333 |
| 2 048 | 8.536 | 1.342 |
| 4 096 | 9.315 | 1.412 |

**And it records which engine *ran*, not which was asked for.** The first
version put the engine that was requested in the conditions, which for a
request that named none was `null` — a measurement whose most important
condition was absent. B65 and D31 are explicit that a timing taken from MCF's
own stand-in measures the stand-in, which is written to be read rather than to
be fast. The engine is now read out of the account the generation returned,
and where it *is* the stand-in both surfaces say so in the sentence beside the
numbers rather than in a footnote.

**What a real repository then said about MCF.** The listing works and the
fitness judgement usually cannot run: the common GGUF publishers ship no
`config.json`, so MCF has no shape to compute a cache size from and answers
that it cannot say which variant would run. That is the honest half of F16 and
it is a poor answer — the GGUF header carries the block and head counts, and
the hub serves ranges, so a few megabytes of prefix would settle it. B-413
carries that rather than leaving the gap as something a user discovers.


## 131 · F131 — The window was a terminal with a mouse pointer over it, and every test passed; one SDL constant was written from memory and named the wrong event (B-409, F129, A6, A11)

**The operator said it was entirely unusable, and they were describing a
decision rather than a defect.** The window shared `mcf_tui`'s character grid
so that one layout could serve both surfaces, and drew it with SDL's 8×8 debug
font. Every test of that code passed, because the tests checked the layout's
arithmetic — that rows were the right width, that the frame closed — and what
was wrong was what the arithmetic produced.

**Nothing could see it.** Checking the appearance meant opening it on a machine
with a display, so it could not be checked in a test, in CI, or by anything
holding only a terminal. Two rounds of *that was terrible* is what a surface
with no observable output costs, and the fix is not a better eye: the window
now draws to a buffer as readily as to a screen, through the same painter, so
an assertion about the interface comes from the interface. Five such tests
exist and two of them found real faults within an hour of being written:

- *Will not run* and *Not measured* were both the warning colour. The two
  states a person most needs to tell apart — this cannot run, and nobody has
  timed this — were the same swatch. No test that read the words would have
  noticed, because the words were right.
- The dark theme's rule was eight steps of lightness from the well it divided.
  A hairline is meant to be quiet; that one was invisible. The palette moved,
  not the threshold.

**A constant was written from memory, and named the wrong event.**
`EVENT_WINDOW_RESIZED` was `0x203`. `0x203` is `SDL_EVENT_WINDOW_HIDDEN`;
resized is `0x206`. The window therefore re-laid itself out whenever it was
hidden and never when it was resized. The provisioned headers were on this
machine the whole time and the number came from recollection instead — which is
A6 in its smallest form, a value carried without the source that fixes it.
Every constant in `sdl.rs` has since been read out of the header, and the
enumeration was counted rather than eyeballed, because the ones that matter had
no explicit value beside them.

**What replaced the grid.** One vendored file — `stb_truetype`, 5,079 lines,
MIT or public domain — and a painter of MCF's own above it: proportional type
at any size, antialiased rounded corners from a single rasterised mask, a
palette named for what each colour does, and six widgets. No toolkit. The
window found Cantarell on this machine and drew with it; the search names
sixteen families and ends with ones nobody picks on looks, because a window
that opens in Liberation Sans is a window that opens.

**And the vocabulary changed, which was most of what made it unusable.** The
first window said *Monitor*, *Diagnostics*, `tokens/second`, `context 32768`,
`Unknown`. Those are the names of MCF's internals. What it says now is *116
words a second — about 29× faster than you can read*, *remembers about 25,000
words*, and *Not measured yet* beside the button that measures it. Nothing was
thrown away: the record still holds milliseconds and tokens, and every figure
the plain sentences replaced is one disclosure down the same page (A1). The two
constants that conversion rests on — 0.75 words to a token, 240 words a minute
— are stated in the interface, because a person told they read at a certain
speed is owed the number (A6).

**The seventh instance's cousin.** [F129](#129--f129--a-second-surface-arrived-and-the-tripwire-watching-for-one-did-not-fire-because-it-was-written-against-four-guesses-at-its-name-b-401-b-072-a22-b16)
and [F130](#130--f130--two-engines-were-provisioned-and-invisible-because-discovery-matched-a-name-and-sixty-four-sentences-a-person-reads-cite-a-document-they-have-never-seen-b-402-b-403-a21-a7-b16)
were guards written against a *name* rather than a shape. This is the same
error against a *number*: a constant recalled rather than read. The remedy is
identical in kind — go to the thing itself — and the reason it keeps recurring
is that recollection is always available and the source always costs a lookup.


## 130 · F130 — Two engines were provisioned and invisible, because discovery matched a name; and sixty-four sentences a person reads cite a document they have never seen (B-402, B-403, A21, A7, B16)

**The operator asked why they had no engines. They had two.**
`llama.cpp` and `llama.cpp-cuda` were both built and sitting on this machine,
and the daemon reported none — because discovery matched `component ==
"llama.cpp"` exactly. The CUDA build, provisioned earlier the same day, was
filtered out by its own name. It was never broken; it was never looked at.

That is the same defect as the surface tripwire in
[F129](#129--f129--a-second-surface-arrived-and-the-tripwire-watching-for-one-did-not-fire-because-it-was-written-against-four-guesses-at-its-name-b-401-b-072-a22-b16),
two days running and the seventh instance overall: **a guard written against
the name of the thing it was looking for, rather than the shape of it.** An
engine is now a prefix with provenance and a server in it, whatever it is
called.

**What a build can compute on is asked, not assumed.** A backend is a property
of how a binary was compiled, and the only honest way to know is to run it: the
CPU build answers `--list-devices` with nothing however many cards are
installed. So MCF asks each engine once, at start — once, because a status
request that starts two processes is a status request that costs something, and
§3.13 makes idle free rather than making being asked expensive. Status now
answers in 16 ms with the devices already known.

    llama.cpp and llama.cpp-cuda ready, on the processor and 1 card

**What runs where is arithmetic, and it is exact.** The weights are a file size
and the cache is a size per token the header states, so the largest window a
device can hold is a division: a power of two, never past the trained context,
never past what the memory holds with headroom. On this machine every model
resolves — a 5 GB model to the card at 32 768, a small one to the card at
32 768, and a state-space model to its own declared limit, because it keeps no
cache that grows and its window costs nothing beyond the weights. Without that
last case it read as *the file does not say how it is shaped*, which is true of
the fields and false about the model.

**And the citations.** The operator's other complaint was that the interface
cites documents. It did: `no inference engine is vendored yet (B-320, D32)`,
printed whether or not an engine was there. Both halves of that sentence were
wrong — the claim and the citation.

Rewriting it was easy; finding the rest was the point. A check that reads every
string literal on a non-comment line in the three crates a person sees, and
looks for the *shape* of a citation rather than a list of the ones that exist,
found **sixty-four** of them across fifteen files.

That is too many to rewrite in one change: several are asserted on by tests
that would have to move with them, and three such tests had to be updated for
the four strings fixed here. So the number is written down and the check is a
ratchet — adding one fails the build, and removing one fails it too, with the
instruction to lower the constant. The count can only go down. B-403 is the row
that takes it to zero.

## Changelog

### Version 102 — the rate halves, and the depth is bounded by memory

F118. The operator's fixed-duration design, tested as a token ladder: the
marginal rate falls 37% and 55% across 128→4096 tokens, far above the machine's
noise and not thermal. The headline rate peaks and then falls, so there is a
length that flatters each model. And the operator found the axis the measurement
left uncontrolled — the engine had allocated 8,192 and 40,960 context for the
two arms, which on one model is a 172 MB difference in memory, nearly twice its
weights.

### Version 101 — the pin, and a rate that was a property of the length

F116 and F117. The benchmark gained a standard question, chosen by measuring 27
vocabularies. Then the operator asked whether verbose models would simply take
longer — they would, and the pin that was supposed to prevent it was declared
and never enforced: SmolLM2 produces five tokens whatever it is asked for. And
measuring that turned up a second thing nobody was looking for: a rate computed
as total over tokens swings 6.6-fold with the length chosen, because 70% of a
short run is fixed cost. A rate is a slope.

### Version 100 — the fifth gate, and a share that can send nothing

F115. A16 names five gated categories and the code enumerated four; the missing
one is publication, the only one that cannot be undone. And `mcf share` finds
that none of the 70 comparisons in this record can travel — 38 have no
established size, and 32 were refused because `mcf bench` never recorded where
its workload came from, which B-203 requires travel from production.

### Version 99 — MCF's own traffic was in the operator's store

F114. Seven hours after the content store began holding anything it held 614
files, and most were the chat-template probe's three constant questions and a
model's answers. §6.8's split is two types and two directories now, and the
category travels with the request because the daemon cannot tell them apart by
looking.

### Version 98 — the load tier found a race in the laboratory

F113. Six runs in a thousand of `engine/server-never-listens` produced `Text
file busy` instead of the failure they declare — the write-then-exec race
against a sibling worker's fork. Diagnosed by counting whole outcomes rather
than rendered ones, which is what hid it on the first pass.

### Version 97 — two absolute rules stop naming checks that do not exist

F112. A6's hole was on a shipped surface — `mcf doctor` printed a p99 with its
sample count and no spread — and is closed by a statistic that cannot be
rendered without its evidence. A22's is enumerated from the control plane's own
enum, passes today, and fails the day a second surface appears.

### Version 96 — what a stale tier cost, measured

F111. The budget tier ran for the first time in two days and failed: the core
binary is 14.3% over its last reading, against a 2% tolerance and comfortably
inside D24's ceiling. Attributed per merge rather than re-baselined on a guess —
80% of it landed a day before the tier fired, which is what the staleness cost.

### Version 95 — a probe for each modality, or a reason

F110. Two probes built — a language's cost on this vocabulary, which asks no
engine, and whether an artifact embeds and does it twice identically — and three
modalities declined in writing with what MCF looked for. The first run of the
embedding probe recorded an engine that did not participate, which is the fifth
of those in four days.

### Version 94 — a rule of thumb that cannot be read as a result

F109. B-380 for the comparison view: a touchstone carries no digit, cannot be
built without saying what MCF has not measured, cannot reach the record, and
expires when the laboratory that would replace it lands.

### Version 93 — an identifier is stable for life

F108. C5 and C6 stop resting on review: a ledger of every published identifier
refuses a removal, a rename, a reordering or a reuse. Writing the citation half
of it found `F80` cited four times with no section behind it, and F80 is written
here from the commit that promised it.

### Version 92 — the oracle's disagreement was the instrument's

F107. The first full run after F103 pinned MCF's own engine reported one
disagreement with room to spare, and it was a false positive: MCF's margin
measured MCF's ranking, the reference's own top-two gap at that step was 0.0199,
and the distribution comparison in the same run put the two engines 0.052 apart
against a floor of 0.20. Two instruments, one question, and the weaker one had
the last word.

### Version 91 — a probe that asks for a shape

F106. B-054 built, and its first real model turned ten *produced no object*
trials into ten *still going when the budget ran out* — MCF interrupting the
model, not the model declining. And B-386's *a probe's outcome is written to the
record* held for one probe of four, because its guard read one function by name.
Third day running for that shape.

### Version 90 — a guarantee that was never on the path

F105. A25 says content lives in a store that is not the record; the store had
no way to hold anything, so nothing was ever put in it and 4 047 entries held
prompts and completions — while `mcf export` printed *no prompt or completion
content, by construction* over them. The guard that computes that claim was
complete about the operator's half of the rule and had never asked about the
model's.

### Version 89 — the tier named the engine and still asked another binary

F104. F103's open question, answered: the tiers inherit the socket. The daemon
honours `--engine stand-in` and its stand-in is its own binary's, so a tier
built a binary and asked a different one — measured, with sixteen corpus
entries landing in a foreign daemon's record. The tiers bring their own socket
now, and the engine line carries the digest of the build that produced it
rather than a version string that never changes.

### Version 88 — the oracle could have been comparing the reference with itself

F103. F102's open question, answered: four scheduled tiers inherited an engine
rather than naming one, and one of them was the oracle — where `mine` and
`theirs` would both have been llama.cpp with a daemon up. And a wedged daemon
made its generation section compare zero models while the headline still read
*agree on all 142 comparisons*.

### Version 87 — a check that answered differently depending on the machine

F102. The conformance corpus ran `mcf run` without naming an engine, so it
reported on whether a daemon was up. The third occurrence of F46's defect, and
it landed exactly where F47's guard cannot see — the guard watches Rust tests
for ambient calls, and a shell tier invoking the built binary makes no such
call. F47 left *whether other tiers have the same dependency* open in writing;
this answers it.

### Version 86 — a call recorded as no call

F101. B-053's probe, on its first real model, reported *no call* five times
out of five for a model that emitted a perfect call every time — it had not
wrapped it in the markers asked for. Two facts folded into one, and the wrong
one survived. The markers a file declares turn out to be the harness's job
rather than the model's, which is A21 in a place nobody had looked.

### Version 85 — how large a model the engine can usefully read

F100. B-384: measured across six models from 24M to 8 billion elements. Time
is no longer what binds — memory is, at about 18 billion elements — and the
reference model is stopped by neither, but by an architecture MCF has not been
taught. Two explanations for the cost curve were wrong before the measurement
settled it.

### Version 84 — threads, measured on both sides

F99. B-366: every product a model performs is faster partitioned, the answer
never moves, and past a product’s optimum more workers is slower — so the
engine spends only what a product earns. Threads did not make it noisier,
which F52 said had to be measured rather than predicted.

### Version 83 — a marker is shown as what it becomes

F79. B-383: every marker-shaped thing in a prompt, with the two questions kept
apart — whether the vocabulary has such a token, and what typing it actually
produces. The second answer is always *ordinary text*, which is D46's safety
property and was F37's expensive surprise.

### Version 82 — the prompt as the model receives it

F78. B-381: `mcf segment` shows where a prompt breaks, fragment by fragment,
with no generation and no judgement. Fifteen tokens for eight Japanese
characters on one vocabulary and four on another — and a silent `unwrap_or`
that had been reporting a whole phrase as one token's work.

### Version 81 — four outcomes and no total

F77. B-200 and B-201: a laboratory result is a sum type whose three
non-readings have nowhere to put a score, a score knows which laboratory it is
on and will not compare across two, and a profile has coverage but no total.

### Version 80 — a run reports as it goes

F76. B-227's missing half: interim lines on standard error, carrying the pair
count and the two arms' medians rather than repeating *not decided* forty
times. A4 already kept what an interrupted run produced.

### Version 79 — a band says what it rested on

F75. B-385 closed: a projection carries the machine conditions of the two
points it was read between, on every surface that renders one. Nothing is
filtered, because filtering needs DEC-007's figure and carrying does not.

### Version 78 — the record caught somebody else's workload

F74. Thirty-three cores of an unrelated .NET test suite made a 400 ms
generation take eighteen seconds, B-217's machine field caught it, and the
projection that reads that history does not yet say so. B-385 registers the
gap; the entries stay, because A1.

### Version 77 — a budget proposes rather than truncating

F73. B-226: a time budget is the operator's to give and produces a proposal
naming both halves, a refusal below a comparison, or a refusal where there is
no measured rate to plan against — never a quietly smaller run.

### Version 76 — work is counted, and minutes are derived

F72. B-224 and B-225: a laboratory's declaration is trials, arms and tokens,
with no field that could hold a duration; the minutes are multiplied out of a
measured band, marked as an estimate, and absent by name where there is no
history.

### Version 75 — the machine either side of a run

F71. B-217's pre-flight, built as a measurement and not as a gate: a comparison
now carries what the machine was doing before it and after it, because the
threshold that would justify a refusal is the figure DEC-007 exists to derive.

### Version 74 — a run that cannot decide names what it competed with

F70. B24's refusal with a name attached, taken on demand and never on a timer.
And five attempts to provoke a real *not decided* at load sixty all reached a
verdict instead, which is a finding about the stopping condition.

### Version 73 — a band predicted before the measurement

F69. A quantization this machine had never benchmarked was projected from the
record at 385.7–404.5 ms and then measured at 397.3. And MCF now scores its own
projection against every measurement it has: 42 of 49 inside, worst miss 150%.

### Version 72 — one file that reproduces one claim

F68. `mcf bundle` builds §II's fourth obligation out of the record. Producing
the first one found that the header's `contains_user_content` had been a
constant `false` since before a comparison recorded the prompt it was asked.

### Version 71 — the first frontier

F67. Seven quantizations of one model, one machine, one sitting, built as seven
paired comparisons against one reference. It is monotone in file size, which is
what F64 predicted: every trial loads the model, and load time goes with bytes.
Also: a size without a direction is not a comparison, and the first run of the
frontier had none.

### Version 70 — a partial run keeps what it produced

F66. `mcf bench`'s runner was discarding every completed pair when one request
failed — A4's own description of its violation. It keeps them now, says what
stopped it, and records no trial for a run that did not happen: a zero-duration
stand-in had been entering the distribution.

### Version 69 — one resident model, or paired interleaving

F65. Making the benchmark warm made it mixed, because one model is resident at
a time and a paired comparison alternates two. A mixed run has no delta to
give; `--cold` and comparing a model with itself are the two uniform ways.

### Version 68 — every trial was cold

F64. What a trial reused is now a condition, and reading it showed that every
benchmark trial loads the model for itself — three fifths of a default trial is
process start, which is why F59's comparison could not see a difference between
two quantizations.

### Version 67 — the recommendation is somewhere else

F63. None of six GGUF repositories publishes a sampling recommendation, and no
acquired model file carries one in its metadata; the recommendation lives in
the base repository the conversion came from. MCF looks in both places it can,
says what it found, and names its own choice as its own.

### Version 66 — the seed set is shown, not assumed

F62. The published set is indistinguishable from a draw ten times larger from
the same stream. The tier nearly cleared it for nothing: MCF's shipped
generation is greedy, and greedy ignores the seed.

### Version 65 — five runs of one comparison

F59 amended. Four of five runs of the same comparison say *no difference as
large as five percent*; the fifth, under a load average of sixty-eight, says
29.2% at a false-alarm rate of 4.7%. One run at one-in-twenty is what
one-in-twenty means, and nothing distinguishes it from §3.27's contention
caveat.

### Version 64 — a trial says what it drew

F61. A trial cannot exist without its seed, and the published set became
arithmetic rather than a list because F55 removed the trial count D19 assumed.
Along the way: EINTR was being classified as a cut-off transfer.

### Version 63 — the clock in the type, applied to writing

F60. Putting the clock in the type stopped a simulated duration being compared
with a real one and not being written down as one. `Measurable` closes it: the
comparison encoder will not take the laboratory's clock, and three of its own
tests stopped compiling.

### Version 62 — a benchmark that cannot fail

F59. `mcf bench` compares two models on an engine that can be timed, refuses a
stand-in by name, and has no verdict that sets a failing exit status. Its first
real comparison — two quantizations of one model on a provisioned llama.cpp —
found a defect: a difference smaller than the resolution asked about was being
reported as a difference.

### Version 61 — the sign test, and a null result on the disk

F57 and F58. The paired verdict is an exact sign test: the resampling it
replaces could not recognize two arms with identical timings, and answered *not
decided* about the clearest null there is. A comparison and a fitment plan are
now record kinds of their own, and neither is a failure.

### Version 60 — a confounded comparison has no delta

F56. An arm is a configuration; `Isolation` says whether a comparison isolated
one variable, none, several, or something MCF has not read enough to judge; and
a confounded comparison's verdict is `None` rather than a number with a warning
beside it.

### Version 59 — the pairing is structural, and it is worth eighty-eight percent

F55. `mcf_bench::compare`: a comparison can only be built from paired,
interleaved, order-randomized trials, and the blocked arrangement is priced at
88% on a real machine. The stopping condition's `Same` branch was asking a
circular question and could only answer one way at small counts; it asks about
power now.

### Version 58 — the first comparison that stopped itself

F54. `mcf_bench::enough`: the rule a comparison carries instead of a repeat
count, since F53 showed a count is a property of the sitting. Three outcomes,
arms paired by construction, and the count reported as part of the answer.

Validated on the question the first benchmark is for — two quantizations of one
model, alternating. Undecided at ten paired trials, decided at seventeen. Not
F51's seven nor F52's fifty: the number was found by the run rather than
brought to it.

### Version 57 — the noise floor is a property of the moment

F53. The last of DEC-007's open pieces, answered without the privilege it was
expected to need: this machine's governor is already at performance and offers
only powersave, so there is nothing to pin. Frequency still moves — nine to one
across cores — but neither it nor load explains the noise; their correlations
with duration change sign between rounds, and a permutation test says a
coefficient that size is what chance gives half the time at twenty samples.

The finding is what happened while looking. Six clean measurements of the same
command gave repeat counts from seven to over a hundred. The noise floor is not
a property of the machine but of the half-hour, which means *at least N
repeats* cannot be the rule. The rule has to be a stopping condition: repeat
until this run's own resampling separates the effect from its own noise, and
report what that took.

And the instrument had a chosen number in it — contamination called at a
correlation of 0.5, which at twenty samples is barely above chance. It is a
permutation test now.

### Version 56 — the engine that will be measured is the noisier one

F52. F51 derived seven repeats from MCF's own engine and said that was the
wrong engine. It was: the provisioned one needs fifty for the same claim, being
about three times noisier.

Two guesses about why, both wrong and both caught by measuring. The daemon path
contributes nothing — 3.6% either side of it. And pinning the server to one
thread, which should have tightened it if threads were the cause, made it five
times worse. Thread count is therefore a condition that must be recorded, and
B-366 cannot predict its own effect on noise.

An earlier reading was contaminated by the machine changing under it and the
instrument reported the range without drawing the conclusion. It now asks
whether duration tracked load and refuses the reading in a sentence nobody can
read past.

### Version 55 — contention moves the level, not the spread

F51. The first measurement the operator's DEC-007 answer asked for: a repeat
count derived rather than chosen. Seven repeats detect a five percent
difference on this machine in its normal state; two percent is not honestly
reachable.

The finding is the shape rather than the size. Sixteen burners made the run
sixty-six percent slower and the spread only twice as wide — so a busy machine
moves the whole distribution coherently rather than mainly adding noise, and no
repeat count removes an error that lands on every repeat in the same direction.
Interleaving does. B-250 asked for that as a principle and now has a number
saying it matters about fifteen times more than the repeat count.

And it settles what it was built to settle: the usable column was taken at a
load average of ten on sixteen cores. An absolute quiet threshold would have
refused a perfectly good measurement.

### Version 54 — the privileged helper could be told where the machine is

F50. Found by preparing to grant the helper the privilege D35 says it needs to
read the processor's energy counter. It accepted a flag that rebased every path
it touches, in the shipped binary and not only in tests — harmless while it held
no privilege, and a local escalation the moment it held any. The seam is a
parameter now and not an argument.

The lesson is about the audit rather than the bug: the repository already
checks that nothing shipped reaches for elevation and that only the laboratory
links the helper, and both passed. The surface was enumerated as three
operations and was really three operations and a root. An enumerated surface is
only enumerated if the arguments are part of the enumeration.

### Version 53 — MCF can check its own engine on a user's machine

F49. `mcf cross-check`: the two engines a user has, compared by teacher forcing
so that the comparison survives the point where their generations part. Four
architectures and a Q2_K file agree within rank 3 of a line at 8; the rotation
deliberately swapped diverges at rank 3388.

The first mutation was the wrong one again — an arm the family never takes —
which is F41's lesson twice in one session and worth the second telling: a
negative control that does not touch the subject looks exactly like one that
does.

### Version 52 — the tie was MCF's, not the model's

F48. B-375 asked for a sharper question than *did the turn end*, to separate
two addressings gemma-3-270m tied on. There was none to find: the two differed
only in a role word, and gemma's template names the losing one exactly once, in
order to rename it. MCF read the template as a bag of words and manufactured a
candidate the file explicitly rejects.

A behavioural question was tried and is recorded for failing on the model it
was for — stopping before the role word and letting the model supply it works
on SmolLM2 and yields a newline on gemma. And with gemma deciding, the
stop-condition probe ran on an unconfigured model for the first time and found
that it had been sending the question as text, which reaches the one engine
that cannot say why it stopped.

### Version 51 — the suite was reporting on the machine it found

F47. B-378 fixed the day it was found, because the suite it undermines is the
one every finding in this document rests on. Where the daemon is, is an input
now rather than something looked up in the middle; the tests pass identically
with a daemon and without, which is the assertion. A table-driven check keeps
the shape from coming back, and a second test asserts the alternatives it names
actually exist.

### Version 50 — MCF's own default was cutting every answer off

F46. The stop-condition probe, and the second applied parameter. MCF allows 32
tokens unless told otherwise and SmolLM2's turns run to 313, so every answer
past the thirty-second token was being ended by MCF rather than by the model —
§3.8's complaint sitting in MCF's own default.

Making the budget apply only where the caller gave none required the
distinction to exist on the wire: `mcf run` substituted its default before
sending, so the daemon could not tell `--limit 32` from silence. D43's rule
that MCF never changes a value under somebody who set it is unenforceable if
the value arrives already substituted.

It also found a gate test whose result depends on whether a daemon happens to
be running, which §3.12 does not allow. B-378.

### Version 49 — the page with no hidden choices had one

F45. `mcf explain` carries the derived configuration now, and said *nothing
here has been probed* to a reader who had probed it. Building that found the
inverse of F44's danger: the check for a moved condition compared the engine a
probe *asked for* against the engine that *resolved*, which are two spellings
of one thing, and reported a machine where nothing had changed as one where the
conditions no longer held. A false divergence looks like diligence, which makes
it the easier error.

### Version 48 — a configuration says whether it still holds

F44. D43's divergence half. The conditions a configuration was taken under can
be checked with no trials and are reported as *the evidence was gathered
elsewhere*, never as a disagreement — F39 measured two engines agreeing on this
question, so a moved engine is not a changed answer. The answer itself is
compared only where a run has just measured it, which is the comparison that
can say wrong.

It also cost eight thousand forward passes to learn the instrument could not
answer. The instrument is asked one token first now, which is a shape worth
keeping: ask whether the instrument can read the result before running the
experiment.

### Version 47 — a model that produced nothing now answers

F43. M3's first exit criterion met: SmolLM2 addressed the way MCF addresses
every model says nothing at all, and addressed the way the chat-template probe
found it answers — with the change written down, recorded, and printed on
every run that uses it.

The act is where D42 and D43 meet: a probe configures nothing, MCF never
reconfigures under a user, so between them there is a person and `--apply` is
where the person is. Inconclusive refuses to apply, which is the one place that
rule has to hold. And the winning addressing is carried out of the probe rather
than rebuilt from its name, because *attributable to a named probe* is only
true if what was applied is what was measured.

### Version 46 — the context is what it says, and asking broke the protocol

F42. The second probe: declared context against the longest prompt the engine
takes. Both models' claims hold. The off-by-one is not a divergence — a
declared context is the whole budget, not the prompt's share — and the probe
leaves a token for the answer and says so.

Asking broke the control protocol. The 64-kibibyte request ceiling predated a
request being able to carry token identifiers, so a legitimate question was cut
off at about four thousand tokens, and the daemon closed on the fragment while
the client was still writing — a connection reset and no reason. Both halves
fixed: a ceiling derived from what a request can honestly be, and a refusal
that names it.

Two smaller errors the work made visible: a well-formed refusal from the engine
was being classified as an unparseable answer, and the probe framework stamped
every result with the first probe's name, which is the provenance failure
B-059 exists to prevent.

### Version 45 — the check could not fail

F41. F40's comparison made durable as an oracle section, and then shown to be
worthless as first written: the negative control returned identical numbers
with the sliding window broken, because the reference stops at 377 tokens and
the window is 512. The check had never reached the mechanism it named.

Fixed by taking the length and naming the artifact rather than avoiding it —
positions where MCF takes the model's end of turn are set aside by that name
and counted. The second mutation was still too weak to see; the third, a
structural break, separates by two orders of magnitude. What the check catches
and what it does not are now both written down.

### Version 44 — the engine agrees for seven hundred positions

F40. B-377 closed. MCF was made to read the reference's own tokens rather than
its own, which is the only way to compare two engines after their greedy
generations have parted. Agreement does not decay with position, and
gemma-3-270m's sliding-window attention is correct past its own 512-token
window — 189 consecutive agreements — which nothing had ever tested because
nothing had ever generated that far.

Two lessons about method. The one alarming number was `ignore_eos` in the
experiment rather than anything in MCF, found by looking up the token instead
of believing the margin. And the oracle's two rules part company on a
per-position test: the margin threshold was calibrated on one parting step per
file and raises false alarms when applied at every position, where the
distribution rule says agreement with room to spare.

### Version 43 — the engine that can be probed

F39. The provisioned engine driven as a server: a turn of identifiers reaches
it, and it says why it stopped. Probing a 270M model goes from tens of minutes
to eight seconds, and the default changed only once both engines were shown to
return the same verdict.

Three times in building it the instrument stood in for the model — a readiness
check one condition short, a token count that measured the engine's chunking
rather than the model's output, and an off-by-one at exactly the boundary F38
turns on. The second of those was the defect F38 is about, living inside the
fix for F38.

It also withdraws a claim: the turn lengths F38 offered as B-375's first
candidate separate gemma-3-270m's tied addressings on one engine and not on
the other, so they were measuring the instrument.

### Version 42 — the probe's answer was upside down

F38. B-374 built: a chat turn assembled from token identifiers, which is what
a chat template actually is. The probe then reported the exact inverse of the
truth — a model that refuses to speak emits its end-of-turn token immediately,
and *did it stop* cannot tell that from a finished answer. Fixed by requiring
the model to have said something, by asking a different question each trial
(greedy sampling made five trials one trial), and by a budget large enough to
reach the end of a turn. B-052 answers for the first time, and agrees with the
file it was checking.

### Version 41 — the first probe found two defects and then refused to answer

F37. `mcf probe` and the chat-template probe. It found that MCF had never used
the stop token it was reading — every generation ran to its budget — and then
that its own first answer was an artifact: MCF cannot put a control token into
a prompt (F26, rightly), so the chat addressings it scored were never applied.
It now reports inconclusive with that reason, which is D42's third state doing
its job.

### Version 40 — the reference model answers

F36. An engine that is a process: the provisioned llama.cpp driven as a
supervised subprocess per generation, its exit classified by the stage it died
in, the daemon standing after each. The reference model — 27B, 16.5 GB, the
artifact §XII named — produced text through MCF in ten and a half seconds.

### Version 39 — what a load costs, apart from running

F35. Loading Qwen3-0.6B costs 0.8 s; the six forward passes of a one-token
request cost 6.8 s. Residency saves the twelfth part of the shortest request on
MCF's own engine, and everything on a faster one. D41 is decided on that.

### Version 38 — distributions against distributions

F34. The oracle compares the reference's top-twenty log-probabilities against
MCF's log-softmax for the same tokens, at the same step, through the same
token ids. Three statistics measured on a clean engine and two known defects;
KL over the top twenty chosen for the width of its separation — clean maximum
0.113 against a subtle defect's median of 0.32 — and the floor set at 0.20.
The instrument's own first failure, comparing step 0 against step 9, is
recorded with it.

### Version 37 — the last schemes, and the widest noise

F33. Four more witnesses close B-364's list: every scheme a corpus file carries
is exercised and compared. The first true Q2_K file parts from the reference
once at 0.320 — evidence says noise, on the coarsest scheme, and the reasoning
is set out — so the threshold moves to 0.40 and the gap it lives in is stated
as the thin thing it has become. A logits comparison is the next instrument.

### Version 36 — a defect the oracle let through

F32. Five small variants acquired to exercise the schemes no file had carried,
and the first thing they found was that MCF's Q3_K decoder read its high-bit
plane off the end — gibberish where the reference says Paris — and the second
was that the oracle had excused it: the broken engine parted from the reference
at step 0 with a margin of 0.449 and was explained by a 0.021 five tokens later.
The rule now takes the margin at the step where they part and nowhere else;
the threshold sits at 0.30. Q5_0 and Q5_1 are decoded, which IQ3_M needed.

### Version 35 — what the first automated provisioning found in two tries

F31. `mcf provision` exists, and its first two runs each found something real.
Rootless podman put its image store under MCF's data home — the content drive,
whose filesystem cannot hold an overlay store — so the store and the prefix now
go to different places on purpose. Then git refused the cloned repository as
foreign-owned from inside the container. Both failures named their exit status
and log and touched nothing in the record.

### Version 34 — two ways to provision, measured

F30. The same component was provisioned through a host prefix and through a
rootless container, and the measurements resolve DEC-052. The host route failed
D39's first condition on its very first use — the oracle was built by a cmake
from a pyenv shim, recorded nowhere, different from what the system says — and
the container route left container storage byte-identical while producing a
binary that runs on the host and agrees with the host-built one exactly.

### Version 33 — the sixth family answers a different question

F29. The bert family embeds, through its own verb — `mcf embed`, vector first
as one JSON line, conditions after — and everything transcribed was checked
against the reference the same day: the tokenizer exactly, the forward pass at
a measured cosine floor of 0.999 that a single swapped normalization falls
through, and the vector's meaning by related-against-unrelated sentences.

The corpus is six for six.

### Version 32 — the mask, past the boundary

F28. The sliding-window mask F27 implemented but could not exercise is now
compared past the boundary: 682 tokens of repetition, and a distinctive name
parked outside the sliding blocks' view with a cue after the filler. Both agree
with the reference token for token. The long comparison is gated behind
`MCF_ORACLE_LONG=1` because the stand-in pays a forward pass per prompt token.

### Version 31 — what a coin-flip looks like, and what a defect looks like

F27. Before the forward pass could be compared at all, a way was needed to tell
two correct implementations disagreeing on a near-tie from one of them being
wrong. That was measured rather than assumed.

Four divergences across fifteen comparisons were noise: margins of 0.040, 0.098,
0.105 and 0.159, in every case the reference choosing exactly MCF's runner-up,
in every case at the smallest margin of that whole generation. The one real
defect diverged at 0.775.

The defect was the sliding-window rotary base. Gemma 3 rotates its sliding
blocks at a different frequency from its global ones, the artifact does not
declare it, and the reference falls back to ten thousand where MCF was using the
million the file states for the others — so five blocks in six were rotating
wrongly. The model still said `Paris.` correctly, which is why nothing before
this caught it. F24 had recorded this as untested ground.

The forward pass is no longer unchecked, for these models and prompts. The
threshold that separates the two cases is provisional, and a defect can still
hide under a near-tie.

### Version 30 — the oracle found a defect on its first run

F26. llama.cpp at a pinned commit, built as a development instrument, compared
against MCF on token identifiers — the one part of an engine that can be checked
exactly, because identifiers are integers.

Twenty-nine of thirty comparisons agreed. The thirtieth was a real defect, and
an invention of MCF's own: a guard that skipped user-defined tokens shorter than
three bytes, on the plausible reasoning that a short token would match inside
ordinary words. No implementation does that, and gemma's vocabulary carries a
token spelled as two literal spaces.

The reference also settled something MCF had guessed: control tokens are left as
text and only user-defined ones are matched, which is both the compatible answer
and the safer one — a prompt should not be able to produce a chat marker by
spelling it.

What found the defect was the tab-and-double-space string, not the sentence.
Gemma answers `Paris.` correctly with the defect present.

### Version 29 — the broken engine read better than the correct one

F25. A mixture of experts runs, read from the reference. The shape comes from
the file's own `expert_count` rather than from the family name — the artifact
declares `llama` and is a mixture of four.

Removing the router entirely, so that every token goes to experts 0 and 1,
produces `Paris. It is located on the River Seine in`. The implementation
transcribed from the reference produces `Paris, France is Paris, Paris is Paris
is`. The broken version reads better.

F20, F22 and F24 each showed that a wrong engine can look right. This shows a
wrong engine looking *better* than a right one, on the same artifact and prompt.
Output quality is not evidence about correctness in either direction, and this
is the strongest case for B-368 that can be made.

### Version 28 — a third architecture, and three habits that fail three ways

F24. Gemma 3 runs, read from the reference rather than inferred. It differs from
llama in five places: two the file states — the normalizations on the way out of
each half of a block — and three it does not — a `GELU` gate, an embedding
scaled by the square root of its width, and the other rotary pairing.

Removing each in turn gives word salad, fluent-and-wrong, and text in another
script. The middle one is the dangerous one, and it is now the third
unobservable habit across two families to fail that way. *Unobservable habit,
got wrong, produces fluent text* looks like the general case rather than a
coincidence.

The division holds up: everything observable is read from the artifact, so
gemma3's normalizations needed no new code path — only two more tensor names.
Everything unobservable is in the one table DEC-053 already governs.

### Version 27 — four expressions where MCF had two

F23. Implementing one more pre-tokenizer meant reading the reference rather than
inferring from a name, and the reading found three defects already shipped:
`qwen2` and `llama-bpe` are different expressions (digits singly against groups
of three), `deepseek-llm` was claimed and is not implemented, and `default` is
not GPT-2's expression but a fourth one.

The first is the instructive one. Qwen3-0.6B tokenizes a numeric prompt
identically before and after the fix, because its merge list never joins digits.
No prompt against that model distinguishes the broken version from the fixed
one. Some facts are about somebody else's software and are read, not measured.

A fourth defect came from the round-trip now run over every corpus vocabulary:
the empty string had stopped encoding as anything for unigram vocabularies, a
regression from adding control-token matching.

And SmolLM2-135M corrects F22's generalization: it is *smaller* than
Llama-160M and answers correctly where that model does not, so the floor for
refereeing an engine is not a parameter count but whether that artifact knows
the answer being asked of it.

### Version 26 — where a model becomes able to referee an engine

F22, which measures what F21 asserted. Two corpus models, run against a correct
engine and one with the rotary pairing swapped — F20's *fluent* defect, chosen
because it is the hard one to see.

At 160M the reader cannot tell the columns apart, because the correct engine
answers three of four prompts wrongly on its own: a model that does not know the
answer cannot be asked whether the engine found it. At 0.6B every prompt is
unmistakable and the correct column is right.

The rule is therefore not *small models are unfit* but *a model must be good
enough to be right about the thing being asked* — which starts below a billion
parameters and costs sixteen seconds. F21's claim that the corpus depends on an
oracle is corrected: the dependency exists at the bottom of the corpus and not
one step up.

### Version 25 — six families for a tenth of one model's bytes

F21. The reference model is 16.5 GB and every engine iteration ran it. Six small
trained models — one per family, one quantization each, 1.4 GB together —
replace it for that job, and §XII is amended (D40) to say which job each
artifact has.

Two of the six run. The other four refuse, each naming what it wants: a
pre-tokenizer, a tokenizer scheme, an expert-routing tensor, an architecture.
Those four refusals are B-365's remaining order of work, read off artifacts
rather than predicted.

The cost is stated rather than glossed: small models cannot referee what F20
turned on, so the corpus depends on B-368's oracle rather than merely preferring
it.

### Version 24 — a second architecture, and two silences that fail differently

F20. Qwen3 needed two things llama does not: a rotary pairing that splits the
head in half rather than taking adjacent pairs, and a normalization of each
query and key head before the rotation. Neither is stated in the file.

Running the engine with each correction present and absent gives four outputs,
and the point of the finding is that they are not four degrees of one failure.
Without the per-head normalization the model emits one token forever. With it
and the wrong rotation, it emits *English* — and an observer without the fourth
row beside it would call that a small model being small.

The normalization was the more serious of the two, because the code for it was
already written: it asked for its weights with `if let Ok(…)`, the manifest
never loaded them, and the error was discarded on every block of every model.
A2 inside an engine written under A2.

### Version 23 — two defects a real model found in an hour

F19. The operator pointed out that their machine already holds a hundred
gigabytes of models, three of which declare the architecture MCF's engine
implements. Running one took an hour and found two defects, neither of which
any test could see.

The tokenizer used SentencePiece's unigram search where these files are read by
a merge algorithm — and in a vocabulary whose byte tokens score zero and whose
words score minus thousands, that search spells every word out one byte at a
time. And `Q6_K` decoded every value correctly into the wrong position, which
in that file corrupts the projection to logits and nothing else.

The third part is the one worth keeping: both had tests, and both tests were
incapable of failing. A uniform block cannot show a permutation, and a fixture
vocabulary of whole words with no intermediate pieces cannot be tokenized by the
correct algorithm at all. A fixture MCF writes to test MCF is shaped by the same
misunderstanding as the code.

### Version 22 — three findings point forward to the one that qualified them

F12, F14 and F15 each recorded that this machine has no C toolchain targeting
musl, and F18 later found it packaged. A reader landing on any of the three
would take the absence for a fact about the world, so each now says where to
read next. Nothing measured is changed — a finding is what was observed when it
was observed — and what each *concluded* is bounded to the condition it was
observed under.

### Version 21 — the wall was a package nobody priced

F18. Three findings — F12 on the engines, F14 on SQLite, F15 on elevation — each
ran into the same absence and treated it as a fact of the world: this machine
has no C compiler targeting musl. Asking the package manager takes a second and
nobody had.

It is packaged: `musl-gcc`, `musl-devel` and `musl-libc-static`, from Fedora's
own repository, about nine megabytes installed. That does not make any candidate
build — a C wrapper says nothing about a C++ project's CMake or about
Oniguruma's configure — and trying it means changing a machine somebody else is
using, which is its operator's call. What it changes is the question B-320 is
answering: not *is this possible* but *are we willing to require a toolchain at
build time*.

### Version 20 — what a hub says when something is gone

F17. §7.38 has asked since it was written what happens when a pinned artifact
decays, and what MCF can detect is a property of the hub rather than of MCF, so
the hub was asked — anonymously, metadata only.

A repository that does not exist answers **401**, with the body *Invalid
username or password*. So *gone*, *private* and *never existed* are one answer,
and no credential MCF could be given tells them apart. A gate, by contrast, is
visible before it bites: the card reads 200 with `"gated":"manual"` while the
file reads 401. And 404 means exactly one thing — a revision that is not in a
repository that is.

The two decays that matter most refuse nothing at all: a relicensing and a
repointed tag are a changed field with a 200 beside it, detectable only by
comparing against what was recorded at acquisition.

### Version 19 — three defects, found by pointing MCF at one real repository

F16. Deciding which quantization of the reference model to pin is the operator's
call, and MCF is supposed to inform it without downloading anything. Pointing
the planner at the real repository for the first time found three defects in
code that had tests.

MCF's JSON reader refused the configuration outright over `1e-06`, because it
was written to keep floating point out of the record — correct property, wrong
mechanism, since the same reader reads documents MCF did not write. The
transformer's fields were under `text_config`, where a multimodal repository
puts them. And counting every block rather than the sixteen that actually cache
would have overstated the KV cache fourfold — the one defect that would have
produced a confident wrong number instead of no number.

All three are fixed, all three have tests, and the plan now classifies all 33
variants without fetching a byte.

### Version 18 — what a machine lets an ordinary user do

F15. §6.32 wants the privileged surface enumerated and §7.39 records that nobody
had enumerated it; documentation cannot, because the answer differs by machine.
So it was asked, without changing anything: pinning cores, bounding memory,
reading temperatures and reading per-process accelerator occupancy need no
privilege; a governor, real-time scheduling, dropping caches and IRQ affinity
do; and reading processor energy — which D11 makes first-class — needs
elevation on this distribution, which is the awkward case §7.39 anticipated.

The probe's own first answer was wrong in the instructive way: `nice -n -5 true`
exits zero having failed, because the shell reports the command's status rather
than whether the priority was applied. Asking the child what it actually ran at
reverses the row. A declaration is not an observation, at the level of a shell
script.

### Version 17 — the index earns its bytes, and SQLite does not

F14. D6 named SQLite the record and D20 later made the record a journal with a
derived index over it; the question of which is which was decided by measuring
both. A replay costs 8 µs an entry — 7.89 s at a million, at every daemon start
— and the derived index answers the same questions in 196 µs from 32 bytes an
entry. SQLite answers them in the same order of time and costs 9.2 MiB of C, 52
seconds of compile, and B-183's static musl container, which cannot be built
here at all for want of a C cross toolchain.

D6 is amended rather than dropped: the journal is the record, the index over it
is derived, and SQLite is a cost to pay if a query arrives that an offset table
cannot answer.

### Version 16 — two writers, one record

F13 added, because the daemon gave MCF a second thing that writes to the record
and §7.37 has never been answered. Eight processes, one file, one whole line per
write, at 400 bytes and 8 KiB and 128 KiB, on tmpfs and on btrfs: sixteen
thousand lines every time, none torn, none interleaved.

That settles the alarming half — two MCF processes do not produce a record
nobody can read, and the load tier now asserts it — and leaves DEC-037's real
question, which turns out to be narrower than it looked. Not *how do we
coordinate writes*, but *who mints an identifier*: each writer counts its own
appends, so two entries can carry the same `EntryId`. Nothing is lost; what is
broken is the assumption that an identifier names one entry.

### Version 15 — the engine's second half, and an expectation removed

F12 completed in the exclusive window. Both candidates build for the ordinary
target; llama.cpp produces 12.9 MiB of static libraries against D24's 40 MiB
ceiling, and a minimal candle program is 1.6 MiB needing only the three
libraries §3a allows.

The result worth recording is the one that did not go as expected. F9.5 chose
the TLS provider on musl: the C candidate could not build for the target
B-183's container uses, and the Rust one could. The same test was expected to
choose the engine — and it does not, because `candle-core` depends on
`tokenizers`, which depends on Oniguruma, which is C. Neither candidate keeps
that check runnable without a cross toolchain, so the engine decision turns on
other things: one upstream against a hundred and forty-three, and a second
tokenizer MCF already owns.

### Version 14 — what an engine would cost, half-measured on purpose

F12 added, and it is the first finding here that reports an *unfinished*
measurement. The cheap half is done: llama.cpp is 35 MiB of one project's C++
under one licence, candle is 152 crates of Rust under many, and this machine has
no musl cross toolchain — the condition that decided the TLS provider in F9.5.

The deciding half needs builds of several minutes on a quiet machine, and the
machine is shared. Rather than guess at the two numbers that matter — whether
either builds for musl, and what each does to D24's 40 MiB ceiling — the finding
says it does not know them, and the script that would fill them in is committed
beside it. A finding that reported an unmeasured half as measured would be the
thing this file exists to prevent.

### Version 13 — a disk fills at the flush

F11 added with B-026. Writing to a full filesystem through a buffered writer
*succeeds*: the buffer takes the bytes and the failure arrives at the flush. A
fetcher that checked its writes and ignored its flush would verify a digest over
bytes that never reached the disk and record an acquisition that did not happen.

Both halves of §3.11 are built on it. A file that will not fit is refused before
a byte moves, with the arithmetic in the refusal rather than a verdict — which
needed the second `unsafe_code` opt-out in the workspace, one `statvfs` call,
checked against `df`. And a filesystem that fills anyway is classified rather
than reported as a general write failure, with `/dev/full` as the scenario.

### Version 12 — what a machine says when there is no network

F10 added, and it closes §7.11 as D33. The measurement that decides it is small
and slightly surprising: a name that will not resolve produces no error kind at
all — `Uncategorized`, no errno — whether the machine has no network, no
resolver, or asked for a name that does not exist. One observation, three
causes, and no way to tell them apart without making a request nobody asked for.

So MCF reports what it saw and names the question it is not answering. The other
three cases *are* distinguishable and are now kept apart, because refused, no
route and silence are three different things for a person to act on. And the
other half of the void needed no experiment: the from-scratch container already
runs everything except acquisition with no network at all.

### Version 11 — the provider that keeps the checks runnable

F9.5 and F9.6 added, and they change the answer 9.4 pointed at. `rustls`'s usual
cryptography is C, and it cannot be built for the musl target B-183's
from-scratch check uses without a cross toolchain nobody has here — so admitting
it would make an existing check runnable in fewer places. A pure-Rust provider
by rustls's own author builds for both targets, compiles fourteen crates rather
than sixteen, and holds a real TLS 1.3 session with the hub, which was run
rather than assumed.

And the ninety megabytes turn out to be avoidable: 9.4 was right that a vendored
tree cannot be *deleted* down to the platform and wrong that it cannot be
filtered. A crate nothing compiles can keep its manifest, its licence and an
empty `lib.rs`. Thirteen megabytes, no C, both targets, offline and locked.

The cost that remains is maturity, and it is stated rather than discovered: MCF
is choosing a young implementation, and it sits behind one trait so that
choosing differently later is one struct.

### Version 10 — what a vendored tree actually costs

F9.4 added after the obvious economy turned out not to exist. Most of a
vendored TLS tree is Windows import libraries a Linux build never compiles, so
the natural move is to trim them — and cargo refuses, because with a vendored
directory it resolves the whole lock graph before compiling any of it. The
honest figure for admitting a TLS stack is therefore about ninety megabytes of
source in the repository rather than fifteen. The fifteen is what gets
compiled, which is the right number for reviewing what MCF ships and the wrong
one for what a checkout costs.

### Version 9 — the transport, measured before it was argued

F9 added. Four backlog items were stopped at the same sentence — *what remains
is the transport* — and the question behind it had never been measured. It is
now: the hub speaks HTTP/1.1, publishes the size and the SHA-256 before the
bytes, serves ranges, and names the revision in the download's own headers.
What MCF lacks is not a protocol, it is TLS, and the honest cost of that is
sixteen crates and 15 MiB rather than the 91 MiB a vendored tree reports —
because most of a vendored tree is Windows import libraries a Linux build never
compiles. The shape that would cost nothing to vendor is the one that demands
`libssl` of the user's machine, and it is in the finding as the control.

### Version 8 — the slope §7.4 was arguing about

F8 added. §7.4 said MCF cannot be faster at matrix multiplication than the
projects that specialize in it, and asked not to be settled on that reading
alone. The prototype measures the slope: the best safe portable Rust it could
maintain is twenty-five to fifty times slower than one core of a *generic*
tuned BLAS on the same machine, and the careful tiling step made it slower than
the one-line reorder — which is what a treadmill looks like from the bottom.

### Version 7 — the signal F5 was missing

F7 added with B-193. A major page fault charged to a child is what tells a
cold-start measurement it went to a device, and the experiment that shows the
signal moving — zero warm, thirty evicted, over the same thirty spawns — runs in
the gating tier, because F3's lesson is that a signal nobody has watched fail is
not known to work.

### Version 6 — the first mutant to survive

F6 added with B-186. The experiment that checked whether the floor refuses
found a real gap while it was at it: nothing tested that MCF's resident-memory
reading is in bytes, and it is a figure `mcf doctor` prints and B-011 asserts
against a ceiling. The mutant is now in the catalogue and the claim has a test.

### Version 5 — a budget that measures the filesystem

F5 added. The first `--all` run failed its cold-start budget by a factor of
two and a half and a later one passed it by two orders of magnitude, on one
machine within the hour; the cause is a `fuseblk` mount whose p99 page-fault
service time is a thousand times its median when its cache is cold. Two things follow and both are
registered as B-193: the storage an artifact is executed from is a condition
nothing records, and D30's signal watches the measuring thread, which for a
cold start is the one thing not doing the work.

### Version 4 — what the new tiers found

F4 added with B-191. Five tiers ran for the first time and four of the six
things they found are about MCF rather than about themselves — a condition that
did not round-trip, a replay whose footprint is proportional to the journal, a
reading that could not be taken in parallel, and a fuzz campaign that had to be
made to say what it reached. The other two are about the tiers, which is what
D10 means by the test of the tests.

### Version 3 — the signal that answers F2, and the one that could not fail

F3 added. It closes what F2 opened and corrects the ground both F1 and F2 stood
on: their cold-start figures were taken on a machine that was never quiet, and
the honest number is thirty times smaller. Neither was wrong about passing; both
were wrong about the number, which is exactly what §3.4's conditions exist to
prevent and what a contaminated one hides.

The second half is a finding about method rather than about MCF. The first
implementation of the new signal read a process's accounting rather than a
thread's, and therefore reported a perfectly clean measurement under any load —
a signal that could not fail. It was caught by running the tier under deliberate
load and checking the verdict *changed*, which is the negative control B-003
established for a lint check, applied to a measurement.

### Version 2 — the budget tier reports that it cannot judge

F2 added. B-011's first run on a real machine established something worth
keeping: the tier's state-class figures assert cleanly, its event-class ones
cannot be asserted at all on a machine somebody is using, and no threshold
fixes that because the machine really is busy. DEC-051 registers the question
rather than the tier quietly loosening until a number passed.

### Version 1 — the adversarial prototype reports

Created to hold F1. B-002's condition is that §7.19 be *amended or confirmed in
writing*, and neither the intent document nor the register is the right place
for the evidence: the first states positions and the second states status, and
a run's conditions and readings are a third kind of thing. Putting them in
either would have made a document say something it is not for.
