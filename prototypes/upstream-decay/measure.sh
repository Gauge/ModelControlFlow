#!/bin/sh
# What a hub says when a pinned artifact is not there any more (DEC-038, §7.38).
#
# MCF pins revisions, which is right, and nothing says what happens when the pin
# goes bad underneath it: a revision withdrawn, a repository gated after
# acquisition, a licence changed, a tag repointed. §7.38 records the question and
# leaves it open, and it cannot be answered from documentation — what a hub
# *does* is a property of that hub, and the answer decides what MCF is able to
# detect at all.
#
# **Nothing here downloads a model.** Every request is metadata: a listing, a
# model card, a `HEAD` of one small file. Four questions, asked of the real hub:
#
#   1. A repository that does not exist.
#   2. A repository that exists and is gated.
#   3. A revision that is not in a repository that is.
#   4. A file that is there, for a control.
#
# The interesting answer is the first one, and it is not the one anybody would
# guess.
#
#   ./prototypes/upstream-decay/measure.sh
#
# Reports a finding.

set -eu

readonly HUB=https://huggingface.co
# Chosen because each is *stably* what it is: a name nobody will ever publish, a
# repository whose gate is famous, a revision that cannot exist, and the
# reference model.
readonly ABSENT=unsloth/this-repository-does-not-exist-mcf-probe
readonly GATED=meta-llama/Llama-2-7b-hf
readonly PRESENT=unsloth/Qwen3.8-27B-GGUF
readonly NO_SUCH_REVISION=0000000000000000000000000000000000000000

command -v curl >/dev/null 2>&1 || { printf 'cannot check: no curl\n' >&2; exit 2; }

ask() {
    printf '  %-58s %s\n' "$2" "$(curl -s -o /dev/null -w '%{http_code}' "$1" || printf 'no answer')"
}

printf '=== what the hub says, anonymously\n'
ask "$HUB/api/models/$ABSENT"                          "a repository that does not exist"
ask "$HUB/api/models/$GATED"                           "a gated repository, its card"
ask "$HUB/api/models/$GATED/tree/main"                 "a gated repository, its file list"
ask "$HUB/$GATED/resolve/main/config.json"             "a gated repository, one file"
ask "$HUB/api/models/$PRESENT"                         "a repository that is there"
ask "$HUB/api/models/$PRESENT/revision/$NO_SUCH_REVISION" "a revision that is not there"

printf '\n=== what a refusal carries\n'
printf '  body of the absent repository: %s\n' \
    "$(curl -s "$HUB/api/models/$ABSENT" | head -c 120)"
printf '  the gated flag, where the card is readable: %s\n' \
    "$(curl -s "$HUB/api/models/$GATED" | tr ',' '\n' | grep '"gated"' | head -1)"

printf '\n=== what changes without any status changing at all\n'
printf '  licence declared by %s: %s\n' "$PRESENT" \
    "$(curl -s "$HUB/api/models/$PRESENT" | tr ',' '\n' | grep -o '"license":"[^"]*"' | head -1)"
printf '  revision now at:      %s\n' \
    "$(curl -s "$HUB/api/models/$PRESENT" | tr ',' '\n' | grep -o '"sha":"[^"]*"' | head -1)"
printf '  A relicensing and a repointed tag are both a field with a different\n'
printf '  value and a 200 beside it: nothing refuses, so nothing is detected\n'
printf '  unless MCF looks and compares.\n'
