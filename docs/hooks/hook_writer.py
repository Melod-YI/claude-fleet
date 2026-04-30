import os, sys, json

hook_input = json.load(sys.stdin)
session_id = hook_input.get("session_id", "unknown")
event_type = hook_input.get("hook_event_name", "unknown")

events_dir = os.path.expanduser("~/.claude-fleet/events")
os.makedirs(events_dir, exist_ok=True)

# SessionEnd 时清理该 session 所有历史文件
if event_type == "SessionEnd":
    for f in os.listdir(events_dir):
        if f.startswith(session_id) and f.endswith(".json"):
            os.remove(os.path.join(events_dir, f))

# 写入事件文件（覆盖同类型旧文件）
file_path = os.path.join(events_dir, f"{session_id}_{event_type}.json")
with open(file_path, "w", encoding="utf-8") as f:
    json.dump(hook_input, f, indent=2, ensure_ascii=False)