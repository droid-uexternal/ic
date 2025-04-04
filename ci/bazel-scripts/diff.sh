#!/usr/bin/env bash

# This script helps us only build bazel targets that have been affected by file
# changes in the branch merging to target branch.
#
# We get the list of files from the diff, query for the ones that are part of any bazel target,
# and then query for the targets that depend on these files.
#
# This scripts is inspired by https://github.com/bazelbuild/bazel/blob/master/scripts/ci/ci.sh

set -euo pipefail

set -x
cd "$(git rev-parse --show-toplevel)"

MERGE_BASE="${MERGE_BASE_SHA:-HEAD}"
# we can't use HEAD here because that is the merge commit which contains the changes of the current HEAD of master
COMMIT_RANGE="$MERGE_BASE..${BRANCH_HEAD_SHA:-}"
DIFF_FILES=$(git diff --name-only "${COMMIT_RANGE}")

if grep -qE "(.*\.bazel|.*\.bzl|\.bazelrc|\.bazelversion|mainnet-canister-revisions\.json|^\.github)" <<<"$DIFF_FILES"; then
    echo "Changes detected in bazel files. Considering all targets." >&2
    echo ${BAZEL_TARGETS:-"//..."}
    exit 0
fi

files=()
for file in $DIFF_FILES; do
    if f="$(bazel query "$file")"; then
        files+=("$f")
    fi
done

if grep -qE ".*\.sh" <<<"$DIFF_FILES"; then
    files+=(//pre-commit:shfmt-check)
fi

if grep -qE ".*\.py" <<<"$DIFF_FILES"; then
    files+=(//pre-commit:ruff-lint)
fi

if grep -qE ".*\.hs" <<<"$DIFF_FILES"; then
    files+=(//pre-commit:ormolu-lint)
fi

if grep -qE ".*\.proto" <<<"$DIFF_FILES"; then
    files+=(
        //pre-commit:protobuf-format-check
        //pre-commit:buf-breaking
    )
fi

if [ ${#files[@]} -eq 0 ]; then
    echo "Changes not detected in bazel targets. No bazel targets to build or test." >&2
    exit 0
fi

if [[ $BAZEL_COMMAND =~ ^build[[:space:]] ]]; then
    TARGETS=$(bazel query "rdeps(//..., set(${files[*]}))")
elif [[ $BAZEL_COMMAND =~ ^test[[:space:]] ]]; then
    EXCLUDED_TAGS=(manual $EXCLUDED_TEST_TAGS)
    EXCLUDED_TAGS=$(
        IFS='|'
        echo "${EXCLUDED_TAGS[*]}"
    )
    TARGETS=$(bazel query "kind(test, rdeps(//..., set(${files[*]}))) except attr('tags', '$EXCLUDED_TAGS', //...)")
else
    echo "Cannot infer command from BAZEL_COMMAND: ${BAZEL_COMMAND:-}" >&2
    exit 1
fi

echo "$TARGETS" | tr '\n' ' ' | sed -e 's/,$//' -e 's/^[[:space:]]*//' -e 's/[[:space:]]*$//'
set +x
