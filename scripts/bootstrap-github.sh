#!/usr/bin/env bash
set -euo pipefail

repository="${OPENWORK_GITHUB_REPOSITORY:-shichenghaoshu/openwork}"
owner="${repository%%/*}"
title="OpenWork Roadmap"

gh auth status >/dev/null
project_number="$(gh project list --owner "$owner" --format json --jq ".projects[] | select(.title == \"$title\") | .number" | head -1)"
if [[ -z "$project_number" ]]; then
  project_number="$(gh project create --owner "$owner" --title "$title" --format json --jq .number)"
fi

create_field() {
  local field_name="$1"
  local options="$2"
  if ! gh project field-list "$project_number" --owner "$owner" --format json --jq '.fields[].name' | grep -Fxq "$field_name"; then
    gh project field-create "$project_number" --owner "$owner" --name "$field_name" --data-type SINGLE_SELECT --single-select-options "$options" >/dev/null
  fi
}

create_field Priority 'P0,P1,P2'
create_field Type 'Feature,Bug,Security,Refactor,Docs,Chore'
create_field Area 'Installer,Control,UI,Gateway,Runtime,Sandbox,Knowledge,Packs,Integrations,Docs,Release'
create_field Size 'S,M,L'
create_field Iteration 'Sprint 0,Sprint 1,Sprint 2,Sprint 3'
create_field 'Target Release' 'v0.1-alpha,v0.1,v0.2,v1.0'
create_field Risk 'Low,Medium,High,Blocked-Upstream'

for issue_number in $(seq 1 30); do
  issue_url="https://github.com/$repository/issues/$issue_number"
  gh project item-add "$project_number" --owner "$owner" --url "$issue_url" >/dev/null 2>&1 || true
done

printf 'Project ready: https://github.com/users/%s/projects/%s\n' "$owner" "$project_number"
