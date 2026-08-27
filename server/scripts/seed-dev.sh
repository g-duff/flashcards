#!/usr/bin/env bash
#
# Hydrates a *development* server with sample data: two beginner Categories
# ("Food" and "Family"), each with ten en/es Vocabulary Entries. Every
# Entry shares one Language Pair (es -> en), so any Entry has nineteen
# siblings to draw four distractors from and is Eligible for a Practice
# Session.
#
# This talks to the public HTTP API only -- it is not a migration and never
# runs against the live database. Point it at a locally running dev server:
#
#   ./server/scripts/seed-dev.sh
#   BASE_URL=http://localhost:8080 ./server/scripts/seed-dev.sh
#
set -euo pipefail

BASE_URL="${BASE_URL:-http://localhost:8080}"

say() { printf '  %s\n' "$1"; }

create_category() {
  local name="$1"
  curl -sS --fail-with-body \
    -X POST "${BASE_URL}/api/categories" \
    -H 'content-type: application/json' \
    -d "{\"name\":\"${name}\"}" >/dev/null
  say "category: ${name}"
}

echo "Seeding ${BASE_URL} ..."

create_category "Food"
create_category "Family"

curl -sS --fail-with-body \
  -X POST "${BASE_URL}/api/vocabulary-entries/bulk" \
  -H 'content-type: application/json' \
  -d @- >/dev/null <<'JSON'
{
  "entries": [
    { "source_language": "es", "source_text": "pan",      "target_language": "en", "target_text": "bread",   "category_name": "Food" },
    { "source_language": "es", "source_text": "agua",     "target_language": "en", "target_text": "water",   "category_name": "Food" },
    { "source_language": "es", "source_text": "manzana",  "target_language": "en", "target_text": "apple",   "category_name": "Food" },
    { "source_language": "es", "source_text": "queso",    "target_language": "en", "target_text": "cheese",  "category_name": "Food" },
    { "source_language": "es", "source_text": "leche",    "target_language": "en", "target_text": "milk",    "category_name": "Food" },
    { "source_language": "es", "source_text": "huevo",    "target_language": "en", "target_text": "egg",     "category_name": "Food" },
    { "source_language": "es", "source_text": "pollo",    "target_language": "en", "target_text": "chicken", "category_name": "Food" },
    { "source_language": "es", "source_text": "arroz",    "target_language": "en", "target_text": "rice",    "category_name": "Food" },
    { "source_language": "es", "source_text": "pescado",  "target_language": "en", "target_text": "fish",    "category_name": "Food" },
    { "source_language": "es", "source_text": "sopa",     "target_language": "en", "target_text": "soup",    "category_name": "Food" },

    { "source_language": "es", "source_text": "madre",    "target_language": "en", "target_text": "mother",      "category_name": "Family" },
    { "source_language": "es", "source_text": "padre",    "target_language": "en", "target_text": "father",      "category_name": "Family" },
    { "source_language": "es", "source_text": "hermana",  "target_language": "en", "target_text": "sister",      "category_name": "Family" },
    { "source_language": "es", "source_text": "hermano",  "target_language": "en", "target_text": "brother",     "category_name": "Family" },
    { "source_language": "es", "source_text": "hijo",     "target_language": "en", "target_text": "son",         "category_name": "Family" },
    { "source_language": "es", "source_text": "hija",     "target_language": "en", "target_text": "daughter",    "category_name": "Family" },
    { "source_language": "es", "source_text": "abuelo",   "target_language": "en", "target_text": "grandfather", "category_name": "Family" },
    { "source_language": "es", "source_text": "abuela",   "target_language": "en", "target_text": "grandmother", "category_name": "Family" },
    { "source_language": "es", "source_text": "tio",      "target_language": "en", "target_text": "uncle",       "category_name": "Family" },
    { "source_language": "es", "source_text": "tia",      "target_language": "en", "target_text": "aunt",        "category_name": "Family" }
  ]
}
JSON

say "20 vocabulary entries (10 Food, 10 Family)"
echo "Done."
