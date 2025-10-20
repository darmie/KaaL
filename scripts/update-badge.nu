#!/usr/bin/env nu
# Update verification badge count in README.md and push to GitHub
#
# Usage: nu scripts/update-badge.nu

def main [] {
    print "🔍 Running verification to get item count..."

    # Run verification and capture output
    let verify_output = (nu scripts/verify.nu | complete)

    if $verify_output.exit_code != 0 {
        print "❌ Verification failed!"
        print $verify_output.stderr
        exit 1
    }

    # Extract total items from output
    let items_line = ($verify_output.stdout | lines | find "All verification passed" | first)
    let items = ($items_line | parse "{before} {items} items verified{after}" | get items.0)
    let modules = ($verify_output.stdout | lines | find "Verified modules:" | first | parse "📊 Verified modules: {count}/{total}" | get count.0)

    print $"✅ Verification complete: ($items) items in ($modules) modules"

    # Update README.md badge
    print "📝 Updating README.md badge..."
    let readme = (open README.md)
    let old_badge_line = ($readme | lines | find "verification-" | find "brightgreen" | first)

    if ($old_badge_line | is-empty) {
        print "❌ Could not find badge line in README.md"
        exit 1
    }

    let new_badge_line = $"![Verification]\(https://img.shields.io/badge/verification-($items)_items_verified-brightgreen)"

    # Replace badge line
    let new_readme = ($readme | str replace $old_badge_line $new_badge_line)
    $new_readme | save -f README.md

    print $"  Updated badge: ($items) items verified"

    # Update verified modules count in README
    let old_modules_line = ($readme | lines | find "Verified Modules:" | first)
    let new_modules_line = $"- **Verified Modules**: ($modules) modules, ($items) items, 0 errors"
    let new_readme2 = ($new_readme | str replace $old_modules_line $new_modules_line)
    $new_readme2 | save -f README.md

    print $"  Updated modules count: ($modules) modules"

    # Check if there are changes
    let status = (git status --porcelain README.md | str trim)

    if ($status | is-empty) {
        print "✨ No changes needed - badge already up to date!"
        exit 0
    }

    # Stage and commit
    print "📦 Committing changes..."
    git add README.md
    git commit -m $"chore\(docs): Update verification badge to ($items) items

Automated badge update after verification run.

Modules: ($modules)
Items: ($items)
Errors: 0"

    # Push to GitHub
    print "🚀 Pushing to GitHub..."
    git push origin main

    print $"✅ Badge updated successfully: ($items) items verified!"
}
