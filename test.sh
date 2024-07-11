jq '. | to_entries | map(select(.key | startswith("s_add")).key)' ref.json
