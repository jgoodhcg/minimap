"""Generate a fake but plausible Clojure monorepo ("quayside", a port-terminal platform laid out as a Polylith workspace).

Output matches the page's data shape: files with path, line count, minimap rows,
outline, namespace, git ages, commit count, and recent commit ids.
"""
import json, math, random, sys, os
from collections import defaultdict

rng = random.Random(4821)
B64 = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"
SCALE = float(sys.argv[2]) if len(sys.argv) > 2 else 1.0

DOMAINS = ["vessel", "voyage", "berth", "booking", "manifest", "container", "cargo", "crane", "yard", "gate",
           "carrier", "terminal", "tariff", "invoice", "customs", "inspection", "hazmat", "pilotage", "towage",
           "appointment", "truck", "driver", "equipment", "reefer", "demurrage", "claim"]
SERVICES = ["geocoder", "ais_feed", "weather", "tide_tables", "pdf_render", "email_gateway", "sms_gateway",
            "object_store", "search_index", "rate_engine", "id_generator", "audit_log", "feature_flags", "currency",
            "calendar", "notifications_hub"]
INTEGRATIONS = ["edi", "ais", "customs_gateway", "port_community", "rail_ops", "payments", "erp_export",
                "weather_bureau", "carrier_api", "truck_gps", "webhook_out"]
EXTRA = ["rules", "events", "queries", "validation", "pricing", "state", "transitions", "notifications", "export",
         "allocation", "scoring", "import", "history", "summary"]
COMPONENTS = ["button", "modal", "drawer", "table", "pager", "tabs", "toast", "tooltip", "badge", "avatar", "card",
              "date_picker", "time_picker", "select", "multi_select", "autocomplete", "checkbox", "radio", "toggle",
              "slider", "stepper", "breadcrumb", "sidebar", "top_nav", "empty_state", "spinner", "skeleton",
              "error_boundary", "file_upload", "map_view", "timeline", "gantt", "calendar_grid", "chip", "menu",
              "popover", "split_pane", "code_block", "markdown", "kpi_tile", "sparkline", "bar_chart", "donut",
              "legend", "search_box", "filter_bar", "column_picker", "bulk_actions", "confirm_dialog", "status_pill",
              "vessel_icon", "berth_bar", "crane_lane", "yard_grid", "container_card", "eta_badge", "weather_chip",
              "tide_chart", "clock", "countdown", "copy_button", "keyboard_hint", "shortcut_help", "pagination",
              "resizable", "virtual_list", "drag_handle", "inline_edit", "form_field", "form_section"]
VERBS = ["allocate", "assign", "build", "calculate", "cancel", "check", "coerce", "compute", "create", "derive", "emit",
         "ensure", "fetch", "find", "format", "group", "handle", "index", "list", "load", "merge", "normalize", "parse",
         "persist", "publish", "rank", "reconcile", "release", "remove", "render", "resolve", "schedule", "score",
         "select", "split", "store", "summarize", "sync", "transform", "update", "upsert", "validate"]
THINGS = ["window", "slot", "plan", "request", "row", "record", "event", "payload", "summary", "total", "fee", "rate",
          "status", "conflict", "candidate", "score", "draft", "limit", "schedule", "eta", "etd", "gap", "lane",
          "block", "hold", "message", "segment", "entry", "line", "item", "batch", "report", "filter", "query",
          "params", "result", "error", "ctx", "config", "window-size", "cutoff", "arrival", "departure", "notice"]
PREDS = ["available?", "valid?", "overdue?", "expired?", "active?", "blocked?", "conflicts?", "allowed?", "empty?"]

files = []
seen = set()


def lognorm(mu, sigma, lo, hi):
    return int(max(lo, min(hi, round(math.exp(rng.gauss(math.log(mu), sigma))))))


def add(path, lang, n=None, **kw):
    if path in seen:
        return
    seen.add(path)
    files.append(dict(p=path, lang=lang, n=n, **kw))


def pick(seq, k):
    k = max(0, min(len(seq), k))
    return rng.sample(seq, k)


def many(count):
    return max(1, int(round(count * SCALE)))


def words(s):
    return s.replace("_", " ")


# ---------------- tree ----------------
# A Polylith workspace: deployable entry points in bases/, one directory per component in components/,
# build artifacts in projects/, and a development project. Migrations, web assets, deployment, analysis, and
# acceptance tests sit beside them.
ROOT = ["README.md", "CHANGELOG.md", "CONTRIBUTING.md", "AGENTS.md", "workspace.edn", "deps.edn", "bb.edn", "tests.edn",
        "shadow-cljs.edn", "package.json", "package-lock.json", ".gitignore", ".tool-versions", ".editorconfig", "justfile"]
for f in ROOT:
    add(f, "root")
for w in ["check", "test-components", "test-bases", "deploy-api", "deploy-worker", "deploy-web", "acceptance-nightly",
          "release", "renovate-automerge", "insights-refresh"]:
    add(f".github/workflows/{w}.yml", "yml")
for f in ["pull_request_template.md", "CODEOWNERS", "renovate.json"]:
    add(f".github/{f}", "root")
add(".clj-kondo/config.edn", "edn")
for h in ["defcomponent", "with-conn", "defjob", "defcommand"]:
    add(f".clj-kondo/hooks/{h}.clj", "clj")
add(".vscode/settings.json", "json")
add(".vscode/extensions.json", "json")
add(".vscode/launch.json", "json", ig=1)

src_files = []


def src(path, lang="clj", **kw):
    add(path, lang, **kw)
    src_files.append(path)


def test_of(path):
    """The test namespace of a source file: src/ becomes test/, and the name gets `_test`."""
    stem, ext = path.rsplit(".", 1)
    return stem.replace("/src/", "/test/", 1) + "_test." + ext


def component(name, files, role, key=None):
    """A JVM component: its deps.edn, and an interface namespace beside its other namespaces."""
    add(f"components/{name}/deps.edn", "root")
    for f in ["interface"] + files:
        src(f"components/{name}/src/quayside/{name.replace('-', '_')}/{f}.clj", role=role, key=key or name)


# components: one per domain, with its SQL in resources
for d in DOMAINS:
    extra = pick(EXTRA, rng.randint(2, 6))
    if d == "berth":
        extra = list(dict.fromkeys(["rules", "allocation", "events", "queries", "scoring"] + extra))
    root = f"components/{d}"
    add(f"{root}/deps.edn", "root")
    for f in ["interface", "core", "spec", "db"] + extra:
        role = "spec" if f == "spec" else "core" if f in ("core", "interface") else "domain"
        src(f"{root}/src/quayside/{d}/{f}.clj", role=role, key=d)
    if d in DOMAINS[:22]:
        add(f"{root}/resources/{d}/queries.sql", "sql-q")
        add(f"{root}/resources/{d}/reports.sql", "sql-q")
for s_ in SERVICES:
    component(s_.replace("_", "-"), ["core", "client"] + pick(["cache", "retry", "schema", "mapper", "fake"], rng.randint(0, 3)),
              "service", key=s_)
for i in INTEGRATIONS:
    if i == "edi":
        continue
    component(i.replace("_", "-"), ["core", "client", "parser", "mapping"] +
              pick(["poller", "webhook", "schema", "fixtures", "backoff"], rng.randint(0, 3)), "service", key=i)
# EDIFACT messages: one namespace per message type
add("components/edifact/deps.edn", "root")
src("components/edifact/src/quayside/edifact/interface.clj", role="core", key="edi")
for seg in ["baplie", "coarri", "codeco", "coprar", "cuscar", "iftmin", "iftsta", "vermas", "movins", "tpfrep",
            "bayplan", "berman", "cusrep", "iftdgn", "copino", "coreor", "desadv", "invoic", "orders", "aperak"]:
    src(f"components/edifact/src/quayside/edifact/messages/{seg}.clj", role="domain", key="edi")
# platform components
component("database", ["core", "pool", "migrate", "tx", "honey", "types", "audit"], "lib", key="db")
component("auth", ["core", "jwt", "oidc", "permissions", "sessions", "api_keys"], "lib", key="auth")
component("event-bus", ["bus", "schemas"], "core", key="events")
for d in pick(DOMAINS, 18):
    src(f"components/event-bus/src/quayside/event_bus/subscribers/{d}.clj", role="domain", key=d)
component("notifications", ["email", "sms", "push", "templates", "digest", "webhooks"], "service", key="notifications")
component("util", ["time", "money", "strings", "maps", "colls", "ids", "json", "csv", "http", "retry", "cache", "metrics",
                   "logging", "validation", "paging", "sorting", "geo", "units", "files", "zip", "crypto", "env", "async",
                   "schema"], "lib")
# shared specs and units, for the JVM and the browser
add("components/specs/deps.edn", "root")
for d in DOMAINS + ["user", "org", "money", "geo", "time", "ids"]:
    add(f"components/specs/src/quayside/specs/{d}.cljc", "cljc-spec", key=d)
for f in pick(DOMAINS, 12):
    add(f"components/specs/test/quayside/specs/{f}_test.cljc", "clj-test", key=f)
add("components/units/deps.edn", "root")
for f in ["time", "money", "ids", "geo", "units", "validation", "errors", "strings", "container_number", "imo"]:
    add(f"components/units/src/quayside/units/{f}.cljc", "cljc", key=f)
for f in ["time", "money", "ids", "container_number", "imo", "units"]:
    add(f"components/units/test/quayside/units/{f}_test.cljc", "clj-test", key=f)
# the browser's component kit: one namespace per component, flat
add("components/ui-kit/deps.edn", "root")
ui = "components/ui-kit/src/quayside/ui_kit"
for c in COMPONENTS:
    add(f"{ui}/{c}.cljs", "cljs-view", key=c)
for f in ["text", "number", "money", "date", "select", "address", "phone", "file", "checkbox_group", "radio_group",
          "tags", "container_number", "vessel_imo", "port_code", "hazmat_class", "temperature", "weight", "dimensions"]:
    add(f"{ui}/forms/{f}.cljs", "cljs-view", key="forms")
for f in ["core", "columns", "sorting", "selection", "export", "filters", "virtual", "cells"]:
    add(f"{ui}/grid/{f}.cljs", "cljs", key="table")
for f in ["line", "bar", "area", "scatter", "heatmap", "gantt", "donut", "axis", "tooltip", "legend"]:
    add(f"{ui}/charts/{f}.cljs", "cljs-view", key="charts")
for f in ["hooks", "i18n", "format", "router", "http", "theme", "icons", "keyboard"]:
    add(f"{ui}/{f}.cljs", "cljs", key="ui")
for c in pick(COMPONENTS, many(30)):
    add(f"components/ui-kit/test/quayside/ui_kit/{c}_test.cljs", "cljs-test", key=c)
add("components/i18n/deps.edn", "root")
for loc in ["en", "es", "nl", "de", "pt", "zh"]:
    for part in ["common", "console", "portal"]:
        add(f"components/i18n/resources/i18n/{loc}/{part}.edn", "edn")

# bases: the API, the worker, reporting, the admin CLI, and three browser apps
B = "bases"
add(f"{B}/api/deps.edn", "root")
api = f"{B}/api/src/quayside/api"
for f in ["main", "system", "config", "routes", "middleware", "coercion", "errors", "openapi"]:
    src(f"{api}/{f}.clj", role="core")
for d in DOMAINS + ["user", "org"]:
    src(f"{api}/handlers/{d}.clj", role="handler", key=d)
for f in ["auth", "tenant", "rate_limit", "request_id", "etag", "cors", "metrics", "tracing"]:
    src(f"{api}/interceptors/{f}.clj", role="lib")
add(f"{B}/api/resources/api/config.edn", "edn")
add(f"{B}/api/resources/api/config.local.edn", "edn", ig=1)
add(f"{B}/api/resources/api/logback.xml", "html")
for t in pick(["booking_confirmed", "booking_cancelled", "berth_assigned", "berth_changed", "eta_updated", "gate_pass",
               "invoice_issued", "invoice_overdue", "password_reset", "welcome", "weekly_digest", "hazmat_notice",
               "customs_hold", "customs_released", "appointment_reminder", "appointment_missed", "claim_opened",
               "claim_resolved", "demurrage_warning", "reefer_alarm", "crane_outage", "yard_full", "driver_invite",
               "carrier_onboarding", "api_key_created", "export_ready", "inspection_scheduled", "tariff_change"], 24):
    add(f"{B}/api/resources/api/email/{t}.html", "html")
for t in ["invoice", "gate_pass", "manifest", "berth_plan", "customs_declaration", "claim_summary", "tariff_sheet",
          "yard_report", "voyage_report", "demurrage_statement"]:
    add(f"{B}/api/resources/api/pdf/{t}.html", "html")
add(f"{B}/worker/deps.edn", "root")
wk = f"{B}/worker/src/quayside/worker"
for f in ["main", "scheduler", "config"]:
    src(f"{wk}/{f}.clj", role="core")
for j in ["berth_planner", "eta_refresh", "demurrage_accrual", "invoice_run", "reefer_monitor", "customs_poll",
          "ais_ingest", "tide_sync", "weather_sync", "digest_mailer", "stale_booking_sweep", "yard_rebalance",
          "crane_schedule", "gate_reconcile", "claim_escalation", "tariff_rollover", "search_reindex", "audit_compact",
          "export_cleanup", "appointment_reminders", "hazmat_review", "carrier_sync", "rail_sync", "session_prune",
          "metrics_rollup", "backup_verify"]:
    src(f"{wk}/jobs/{j}.clj", role="job", key=j.split("_")[0])
add(f"{B}/reporting/deps.edn", "root")
for r in ["berth_utilization", "crane_productivity", "yard_dwell", "gate_throughput", "vessel_turnaround",
          "demurrage_aging", "invoice_aging", "revenue_by_carrier", "customs_holds", "hazmat_register", "reefer_uptime",
          "appointment_no_shows", "truck_turn_time", "claims_summary", "tariff_exceptions", "booking_backlog",
          "voyage_delays", "equipment_utilization", "pilotage_hours", "towage_usage", "carrier_scorecard",
          "terminal_kpis", "manifest_discrepancies", "inspection_outcomes", "cargo_mix", "container_moves"]:
    src(f"{B}/reporting/src/quayside/reporting/{r}.clj", role="report", key=r.split("_")[0])
for f in ["main", "render", "schedule", "export"]:
    src(f"{B}/reporting/src/quayside/reporting/{f}.clj", role="core")
add(f"{B}/admin/deps.edn", "root")
adm = f"{B}/admin/src/quayside/admin"
src(f"{adm}/main.clj", role="core", key="admin")
CMD_GROUPS = {
    "data": ["dump", "restore", "seed", "reindex", "replay"],
    "users": ["add", "disable", "invite", "roles", "sessions", "audit"],
    "tenants": ["add", "configure", "flags", "rotate_keys"],
    "berths": ["plan_preview", "plan_apply", "conflicts", "utilization"],
    "billing": ["invoice_preview", "invoice_void", "tariff_import", "tariff_diff", "demurrage_recalc"],
    "feeds": ["edifact_parse", "edifact_send", "customs_status", "ais_tail", "weather_backfill", "tide_backfill"],
    "system": ["status", "env_diff", "secrets_check", "bench", "doctor", "repl"]}
for g, cmds in CMD_GROUPS.items():
    for c in cmds:
        src(f"{adm}/{g}/{c}.clj", role="lib", key=g)
for f in ["output", "table", "colors", "config", "prompt"]:
    src(f"{adm}/term/{f}.clj", role="lib", key="admin")


def app(name, doms, views_per):
    add(f"{B}/{name}/deps.edn", "root")
    base = f"{B}/{name}/src/quayside/{name}"
    for f in ["core", "db", "router", "events", "subs", "effects", "interceptors"]:
        add(f"{base}/{f}.cljs", "cljs", key=name, role="core")
    for d in doms:
        add(f"{base}/events/{d}.cljs", "cljs-events", key=d)
        add(f"{base}/subs/{d}.cljs", "cljs-subs", key=d)
        vs = ["page"] + pick(["table", "form", "detail", "filters", "timeline", "map", "bulk", "planner", "history"], rng.randint(*views_per))
        if d == "berth" and name == "console":
            vs = list(dict.fromkeys(["page", "planner", "table", "detail"] + vs))
        for v in vs:
            add(f"{base}/views/{d}/{v}.cljs", "cljs-view", key=d)
    for t in pick(doms, int(len(doms) * 0.6)):
        add(f"{B}/{name}/test/quayside/{name}/events/{t}_test.cljs", "cljs-test", key=t)


app("console", DOMAINS[:24], (1, 4))
app("portal", ["booking", "invoice", "container", "appointment", "claim", "tariff", "customs", "manifest", "voyage",
               "vessel", "truck", "driver", "demurrage", "reefer"], (1, 3))
app("mobile", ["appointment", "gate", "truck", "container", "profile", "inbox", "route", "checklist"], (1, 2))

# tests for the JVM sources of the components and bases
for p in src_files:
    if rng.random() < 0.58 or "/berth/" in p:
        add(test_of(p), "clj-test", key=p)

# projects: what each deployable artifact bundles
for pr in ["api-server", "worker", "reporting", "admin-cli", "console-web", "portal-web", "mobile-web"]:
    add(f"projects/{pr}/deps.edn", "root")
    add(f"projects/{pr}/README.md", "md")
    if not pr.endswith("-cli"):
        add(f"projects/{pr}/Dockerfile", "root")
    if pr.endswith("-web"):
        add(f"projects/{pr}/shadow-cljs.edn", "edn")

# the development project: the REPL, seeds, scenarios, and component cards
add("development/src/user.clj", "clj", role="lib")
add("development/src/dev/seed.clj", "clj", role="lib")
for s_ in ["busy_terminal", "hazmat_week", "storm_delay", "customs_backlog", "crane_outage", "empty_port",
           "holiday_peak", "reefer_heavy"]:
    add(f"development/src/dev/scenarios/{s_}.clj", "clj", role="lib")
for c in pick(COMPONENTS, many(44)):
    add(f"development/cards/quayside/cards/{c}_cards.cljs", "cljs-view", key=c)
for t in ["vessels", "carriers", "terminals", "berths", "cranes", "tariffs", "users", "orgs", "roles", "hazmat_classes",
          "ports", "equipment_types"]:
    add(f"development/resources/seeds/{t}.edn", "edn")
for d in pick(DOMAINS, 20):
    add(f"development/resources/fixtures/{d}.edn", "edn")

# migrations: one up and one down script each, ordered by timestamp
MIGS = many(160)
for i in range(MIGS):
    d = rng.choice(DOMAINS + ["user", "org", "role", "audit", "session", "api_key"])
    act = rng.choice(["create", "add-column-to", "index", "backfill", "alter", "drop-column-from", "rename-in"])
    stamp = f"{2018 + i * 7 // 100}{(i * 5) % 12 + 1:02d}{(i * 11) % 28 + 1:02d}{(i * 37) % 24:02d}{(i * 13) % 60:02d}"
    for way in ["up", "down"]:
        add(f"migrations/{stamp}-{act}-{d.replace('_', '-')}s.{way}.sql", "sql-mig", mig=i)

# static web assets
for f in ["index", "console", "portal", "mobile"]:
    add(f"assets/html/{f}.html", "html")
for f in ["base", "reset", "tokens", "layout", "grid", "forms", "buttons", "dialogs", "charts", "print", "console",
          "portal", "mobile", "gantt", "yard", "maps", "utilities", "dark"]:
    add(f"assets/css/{f}.css", "css")
ICON_GROUPS = ["nav", "status", "vessel", "cargo", "equipment", "weather", "finance", "people", "alerts", "actions"]
for g in ICON_GROUPS:
    for i in range(rng.randint(4, 9)):
        add(f"assets/icons/{g}/{rng.choice(COMPONENTS + DOMAINS)}_{i:02d}.svg", "bin")
for i in range(16):
    add(f"assets/images/{rng.choice(['hero', 'empty', 'error', 'logo', 'onboard', 'crane', 'vessel'])}_{i}.png", "bin")
for f in ["plex_sans_400", "plex_sans_600", "plex_mono_400", "plex_mono_600", "icons", "digits"]:
    add(f"assets/fonts/{f}.woff2", "bin")

# deployment: Terraform modules and stacks, and a Helm chart per service
for m in ["network", "database", "cache", "queue", "storage", "cdn", "dns", "monitoring", "alerts", "secrets", "iam",
          "compute", "search", "backups"]:
    for f in ["main", "variables", "outputs"] + (["README"] if rng.random() < 0.4 else []):
        add(f"deploy/terraform/modules/{m}/{f}.{'md' if f == 'README' else 'tf'}", "md" if f == "README" else "tf", key=m)
for e in ["dev", "staging", "prod"]:
    for f in ["main.tf", "variables.tf", "backend.tf", "terraform.tfvars"]:
        add(f"deploy/terraform/stacks/{e}/{f}", "tf")
for svc in ["api", "worker", "reporting", "console", "portal", "mobile", "edifact-gateway", "ais-ingest", "search"]:
    add(f"deploy/helm/{svc}/Chart.yaml", "yml", key=svc)
    add(f"deploy/helm/{svc}/values.yaml", "yml", key=svc)
    for f in ["deployment", "service", "configmap"] + (["hpa"] if rng.random() < 0.4 else []):
        add(f"deploy/helm/{svc}/templates/{f}.yaml", "yml", key=svc)

# insights: saved queries, notebooks, and reference tables for analysis
for f in ["README.md", "requirements.txt"]:
    add(f"insights/{f}", "md" if f.endswith(".md") else "txt")
for area in ["operations", "finance", "customs", "equipment", "customers", "safety", "people", "berths", "yard",
             "gate", "carriers", "forecasts"]:
    for i in range(rng.randint(3, 9)):
        thing = rng.choice(THINGS).replace("-", "_")
        add(f"insights/queries/{area}/{rng.choice(DOMAINS)}_{thing}s.sql", "sql-model", key=area)
for p_ in pick(["ais_backfill", "weather_join", "tide_join", "gate_logs", "invoice_export", "carrier_feed",
                "customs_feed", "crane_telemetry", "reefer_telemetry", "yard_positions", "truck_gps", "rail_feed",
                "edi_archive", "search_logs", "api_logs", "email_events", "sms_events", "push_events", "claims_feed",
                "tariff_feed", "fx_rates", "holidays", "port_calls", "vessel_registry", "dq_checks", "lineage",
                "snapshot_export", "anomaly_scan", "forecast_eta", "forecast_dwell"], 30):
    add(f"insights/notebooks/{p_}.py", "py", key=p_.split("_")[0])
for s_ in ["port_codes", "carrier_codes", "hazmat_classes", "equipment_types", "country_codes", "holidays",
           "tariff_codes", "time_zones", "currencies", "berth_catalog", "crane_catalog", "status_map"]:
    add(f"insights/reference/{s_}.csv", "csv")

# acceptance: Gherkin features per app and the step definitions that drive a browser
for appn, doms in [("console", DOMAINS[:22]), ("portal", ["booking", "invoice", "container", "appointment", "claim",
                                                         "tariff", "customs", "manifest", "voyage", "vessel"]),
                   ("mobile", ["appointment", "gate", "truck", "container", "profile"])]:
    for d in doms:
        for flow in pick(["create", "edit", "cancel", "search", "filter", "export", "bulk", "permissions",
                          "happy_path", "validation", "offline", "history"], rng.randint(1, 4)):
            add(f"acceptance/features/{appn}/{d}/{flow}.feature", "feature", key=d)
add("acceptance/deps.edn", "root")
for f in ["browser", "hooks", "world", "api", "config"]:
    add(f"acceptance/src/quayside/acceptance/support/{f}.clj", "clj", role="lib", key="acceptance")
for pg in pick(COMPONENTS[:30] + DOMAINS, 25):
    add(f"acceptance/src/quayside/acceptance/steps/{pg}.clj", "clj", role="lib", key=pg)

# docs
decision_titles = ["record-decisions", "use-clojure-on-the-jvm", "use-postgres", "adopt-polylith", "adopt-integrant",
                   "reitit-for-routing", "re-frame-for-browser-apps", "honeysql-over-orm", "tenant-per-schema",
                   "edifact-parser-in-house", "ais-via-vendor-feed", "shadow-cljs", "event-bus-in-process",
                   "jobs-on-quartz", "search-with-opensearch", "terraform-modules", "helm-per-service", "feature-flags",
                   "berth-planner-scoring", "manual-allocations-win", "money-as-cents", "utc-everywhere",
                   "mobile-app-pwa", "api-versioning", "idempotency-keys", "webhook-signing", "audit-log-schema",
                   "soft-deletes", "pdf-rendering", "email-provider", "sms-provider", "rate-engine-rewrite",
                   "demurrage-accrual-nightly", "customs-polling", "reefer-alarms", "gherkin-acceptance-tests",
                   "bb-for-tools", "open-telemetry"]
for i, t in enumerate(decision_titles):
    add(f"docs/decisions/{i + 1:03d}-{t}.md", "md-adr", adr=i)
for r in ["berth-planner-stuck", "edifact-backlog", "ais-feed-down", "db-failover", "restore-from-backup",
          "rotate-secrets", "invoice-run-failed", "customs-gateway-errors", "reefer-alarm-storm", "crane-outage",
          "deploy-rollback", "search-reindex", "queue-lag", "cert-expiry", "tenant-onboarding", "tenant-offboarding",
          "data-export-request", "incident-comms", "on-call-handbook", "load-shedding", "weather-emergency", "port-closure"]:
    add(f"docs/playbooks/{r}.md", "md")
for g in ["getting-started", "repl-workflow", "testing", "migrations", "berth-planner", "edifact-primer",
          "domain-glossary", "workspace-layout", "insights", "release-process", "code-review", "security"]:
    add(f"docs/guides/{g}.md", "md")
for d in ["system-context", "components", "berth-planner-flow", "edifact-flow", "tenancy", "deploy"]:
    add(f"docs/diagrams/{d}.svg", "bin")
for s_ in ["setup.sh", "lint.sh", "format.sh", "check_deps.clj", "new_migration.clj", "new_decision.clj", "release.sh",
           "changelog.clj", "db_local.sh", "open_pr.sh", "sync_i18n.clj", "sizes.clj", "hooks_install.sh", "ci_cache.sh",
           "port_forward.sh"]:
    add(f"tools/{s_}", "sh" if s_.endswith(".sh") else "clj")

# ---------------- names ----------------
NS_ROOTS = [r"^(?:bases|components)/[^/]+/(?:src|test)/(.*)$", r"^development/(?:src|cards)/(.*)$",
            r"^acceptance/src/(.*)$", r"^(?:\.clj-kondo|tools)/(.*)$"]


def ns_of(p):
    import re
    for pat in NS_ROOTS:
        m = re.match(pat, p)
        if m:
            rest = m.group(1).rsplit(".", 1)[0]
            return rest.replace("/", ".").replace("_", "-")
    return None


PHRASES = [
    "allocate-{k}-window", "release-{k}-window", "find-free-windows", "window-conflicts?", "overlapping-windows",
    "score-candidate", "rank-candidates", "best-candidate", "apply-tariff", "calculate-fee", "compute-total",
    "round-to-cents", "surcharge-for", "->row", "row->{k}", "->{k}-summary", "normalize-eta", "eta-drift-minutes",
    "late?", "overdue?", "within-cutoff?", "load-{k}", "load-{k}s-by-terminal", "save-{k}!", "upsert-{k}!",
    "delete-{k}!", "list-{k}s", "get-{k}", "find-{k}-by-ref", "validate-{k}", "valid-{k}?", "coerce-params",
    "handle-{k}-created", "handle-{k}-updated", "emit-{k}-event", "publish-{k}-changed", "{k}-status",
    "transition-{k}", "allowed-transitions", "can-cancel?", "cancel-{k}!", "hold-{k}!", "clear-hold!", "hold-reasons",
    "notify-carrier", "notify-{k}-watchers", "build-{k}-email", "render-{k}-pdf", "export-{k}s-csv", "import-{k}s",
    "parse-{k}-message", "group-by-terminal", "group-by-day", "summarize-{k}s", "{k}-kpis", "daily-{k}-counts",
    "sync-{k}s", "reconcile-{k}s", "merge-duplicates", "draft-limit-ok?", "crane-blocked?", "maintenance-blocks",
    "tide-window", "weather-hold?", "default-page-size", "max-window-hours", "{k}-schema", "{k}-defaults",
    "with-{k}-tx", "wrap-{k}-errors", "retry-policy", "backoff-ms", "cache-key", "cached-{k}", "{k}-by-id",
    "index-{k}s", "{k}-changes", "diff-{k}", "stale-{k}s", "archive-{k}!", "restore-{k}!", "{k}-history",
    "latest-{k}", "{k}-query", "where-clause", "order-by-clause", "paginate", "enrich-{k}", "attach-terminal",
    "assoc-totals", "select-columns", "handler", "routes", "start!", "stop!", "run-once!", "schedule!"]
EVENTS = ["load-{k}s", "{k}s-loaded", "load-failed", "save-{k}", "{k}-saved", "save-failed", "set-filter",
          "clear-filters", "select-{k}", "open-{k}-modal", "close-modal", "sort-by", "set-page", "bulk-select",
          "refresh", "poll-{k}s", "stop-polling", "undo", "delete-{k}", "{k}-deleted", "init-{k}-form", "set-field"]
SUBS = ["{k}s", "selected-{k}", "filters", "loading?", "error", "page", "sorted-{k}s", "visible-{k}s", "counts",
        "can-save?", "dirty?", "form", "form-errors", "total-{k}s"]
SPECS = ["{k}-id", "{k}", "status", "eta", "etd", "terminal-id", "created-at", "updated-at", "reference", "notes",
         "carrier-id", "voyage-id", "{k}-type", "{k}-ref", "quantity", "weight-kg", "window", "starts-at", "ends-at"]


def phrase(key, bank):
    k = (key or rng.choice(DOMAINS)).split("/")[-1].replace(".clj", "").replace("_", "-")
    return rng.choice(bank).format(k=k)


def fname(key):
    k = (key or rng.choice(DOMAINS)).split("/")[-1].replace(".clj", "").replace("_", "-")
    if rng.random() < 0.75:
        return rng.choice(PHRASES).format(k=k)
    r = rng.random()
    v, t = rng.choice(VERBS), rng.choice(THINGS)
    if r < 0.35:
        return f"{v}-{k}-{t}"
    if r < 0.55:
        return f"{v}-{t}"
    if r < 0.7:
        return f"{k}-{t}"
    if r < 0.8:
        return f"{t}-{rng.choice(PREDS)}"
    if r < 0.9:
        return f"->{k}-{t}"
    return f"{k}->{t}"


# ---------------- synthesis ----------------
FORCE = [None]
MIN_N = {"components/berth/src/quayside/berth/rules.clj": 430, "bases/worker/src/quayside/worker/jobs/berth_planner.clj": 380,
         "bases/console/src/quayside/console/views/berth/planner.cljs": 270,
         "components/berth/src/quayside/berth/allocation.clj": 240}


def synth_clj(ns, n_target, kind_fn, name_fn, body_mu=8, doc_p=0.4, deep=False):
    n_target = FORCE[0] or n_target
    L, O = [], []
    L.append((0, 4 + len(ns or "user")))
    L.append((2, rng.randint(9, 11)))
    for _ in range(rng.randint(2, 14)):
        L.append((13, rng.randint(18, 64)))
    L.append((0, 0))
    used = set()
    while len(L) < n_target:
        kind = kind_fn()
        name = name_fn()
        tries = 0
        while name in used and tries < 30:
            name = name_fn(); tries += 1
        if name in used:
            name = f"{name.rstrip('?!')}-{rng.choice(THINGS)}"
        used.add(name)
        O.append([len(L) + 1, kind, name])
        L.append((0, len(kind) + len(name) + 2 + rng.randint(0, 18)))
        if kind in ("defn", "defn-", "defmacro", "deftest") and rng.random() < doc_p:
            for _ in range(rng.randint(1, 3)):
                L.append((2, rng.randint(28, 74)))
        if kind in ("defn", "defn-", "defmacro"):
            L.append((2, rng.randint(4, 34)))
        body = lognorm(body_mu, 0.7, 1, 60) if kind != "def" else rng.randint(1, 6)
        ind = 2
        for _ in range(body):
            step = rng.choice([-4, -2, -2, 0, 0, 0, 2, 2, 2, 4] if deep else [-2, -2, 0, 0, 0, 2, 2, 4])
            ind = max(2, min(20 if deep else 14, ind + step))
            L.append((ind, ind + rng.randint(5, 60 if not deep else 44)))
        L.append((0, 0))
    if rng.random() < 0.3 and kind_fn() != "deftest":
        L.append((0, 8))
        for _ in range(rng.randint(3, 12)):
            L.append((2, rng.randint(10, 50)))
    return L, O


def weighted(pairs):
    x, acc = rng.random(), 0
    for k, w in pairs:
        acc += w
        if x <= acc:
            return k
    return pairs[-1][0]


def synth_sql(n, style):
    L, O = [], []
    while len(L) < n:
        if style == "mig":
            t = rng.choice(DOMAINS) + "s"
            O.append([len(L) + 1, "table", t])
            L.append((0, 14 + len(t)))
            for _ in range(rng.randint(3, 14)):
                L.append((2, rng.randint(16, 52)))
            L.append((0, 2)); L.append((0, 0))
            if rng.random() < 0.6:
                L.append((0, rng.randint(40, 80))); L.append((0, 0))
        else:
            cte = f"{rng.choice(DOMAINS)}_{rng.choice(THINGS).replace('-', '_')}s"
            O.append([len(L) + 1, "cte", cte])
            L.append((0, 10 + len(cte)))
            L.append((4, 10))
            for _ in range(rng.randint(3, 16)):
                L.append((8, rng.randint(12, 60)))
            for _ in range(rng.randint(1, 5)):
                L.append((4, rng.randint(12, 60)))
            L.append((0, 3)); L.append((0, 0))
    return L, O


def synth_md(n, title, heads=None):
    L, O = [(0, len(title) + 2)], [[1, "h1", title]]
    L.append((0, 0))
    heads = heads or ["Overview", "Setup", "Usage", "Examples", "Notes", "Links"]
    hi = 0
    while len(L) < n:
        r = rng.random()
        if r < 0.18:
            h = heads[hi % len(heads)]; hi += 1
            lvl = "h2" if rng.random() < 0.75 else "h3"
            O.append([len(L) + 1, lvl, h])
            L.append((0, len(h) + (3 if lvl == "h2" else 4))); L.append((0, 0))
        elif r < 0.55:
            for i in range(rng.randint(1, 5)):
                L.append((0, rng.randint(60, 100)))
            L.append((0, rng.randint(20, 70))); L.append((0, 0))
        elif r < 0.8:
            for _ in range(rng.randint(2, 7)):
                ind = 0 if rng.random() < 0.75 else 2
                L.append((ind, ind + rng.randint(20, 84)))
            L.append((0, 0))
        else:
            L.append((0, 7))
            for _ in range(rng.randint(2, 12)):
                ind = rng.choice([0, 0, 2, 2, 4])
                L.append((ind, ind + rng.randint(8, 60)))
            L.append((0, 3)); L.append((0, 0))
    return L, O


def synth_nested(n, maxd=5, step=2, lo=6, hi=48, keyout=None):
    L, O = [], []
    d = 0
    while len(L) < n:
        d = max(0, min(maxd, d + rng.choice([-2, -1, -1, 0, 0, 1, 1])))
        ind = d * step
        if d == 0 and keyout:
            k = keyout()
            O.append([len(L) + 1, "key", k])
            L.append((0, len(k) + 1))
        else:
            L.append((ind, ind + rng.randint(lo, hi)))
        if d == 0 and rng.random() < 0.3:
            L.append((0, 0))
    return L, O


def synth_blocks(n, head_kind, head_name, body_ind=2, deep=4):
    L, O = [], []
    while len(L) < n:
        nm = head_name()
        O.append([len(L) + 1, head_kind, nm])
        L.append((0, len(nm) + 12))
        for _ in range(rng.randint(2, 14)):
            ind = rng.choice([body_ind] * 3 + [body_ind + 2, min(body_ind + deep, 12)])
            L.append((ind, ind + rng.randint(8, 54)))
        L.append((0, 1)); L.append((0, 0))
    return L, O


def lines_for(f):
    p, lang = f["p"], f["lang"]
    name = p.rsplit("/", 1)[-1]
    key = f.get("key")
    if lang == "bin":
        return None
    if name == "package-lock.json":
        return synth_nested(11800 + rng.randint(0, 900), maxd=6, lo=6, hi=40)
    if name == "subject.txt":
        return [(0, rng.randint(24, 64))], []
    if name == "AGENTS.md":
        return synth_md(142, "AGENTS.md", ["Overview", "Setup", "Workspace layout", "Commands", "Testing", "Conventions",
                                           "Releases"])
    if name.endswith(".md"):
        title = name[:-3].replace("-", " ").replace("_", " ").strip().capitalize() or "Readme"
        if lang == "md-adr":
            title = title[5:].capitalize() if title[:4].isdigit() else title
            return synth_md(lognorm(70, 0.4, 25, 220), title, ["Status", "Context", "Decision", "Consequences", "Alternatives"])
        if "/playbooks/" in p:
            return synth_md(lognorm(90, 0.5, 25, 300), title, ["Symptoms", "Impact", "Diagnosis", "Fix", "Rollback", "Escalation"])
        return synth_md(lognorm(85, 0.6, 12, 420), title, ["Overview", "Setup", "Commands", "Architecture", "Conventions",
                                                            "Testing", "Deployment", "Troubleshooting", "Glossary"])
    if lang in ("clj", "cljs", "cljc", "clj-test", "cljs-test", "cljs-events", "cljs-subs", "cljs-view", "cljc-spec", "root") and \
            name.rsplit(".", 1)[-1] in ("clj", "cljs", "cljc"):
        ns = ns_of(p)
        role = f.get("role", "")
        if lang in ("clj-test", "cljs-test"):
            base = (key or "").rsplit("/", 1)[-1].replace(".clj", "")
            return synth_clj(ns, lognorm(95, 0.65, 18, 900), lambda: "deftest", lambda: fname(base).lstrip("->").rstrip("?!") + "-test", body_mu=10)
        if lang == "cljs-events":
            return synth_clj(ns, lognorm(95, 0.6, 20, 700), lambda: weighted([("rf/reg-event-fx", .7), ("rf/reg-event-db", .25), ("defn-", .05)]),
                             lambda: "::" + phrase(key, EVENTS), body_mu=7, doc_p=0.05)
        if lang == "cljs-subs":
            return synth_clj(ns, lognorm(55, 0.5, 12, 300), lambda: "rf/reg-sub", lambda: "::" + phrase(key, SUBS), body_mu=4, doc_p=0)
        if lang == "cljs-view":
            return synth_clj(ns, lognorm(110, 0.6, 16, 900), lambda: weighted([("defn", .78), ("defn-", .17), ("def", .05)]),
                             lambda: rng.choice([f"{(key or 'view').replace('_', '-')}-{rng.choice(['panel', 'row', 'header', 'cell', 'list', 'form', 'actions', 'summary', 'empty', 'toolbar', 'badge', 'modal'])}", fname(key)]),
                             body_mu=11, deep=True, doc_p=0.1)
        if lang == "cljc-spec" or role == "spec":
            return synth_clj(ns, lognorm(80, 0.5, 14, 500), lambda: weighted([("s/def", .8), ("defn", .15), ("def", .05)]),
                             lambda: "::" + phrase(key, SPECS), body_mu=2, doc_p=0)
        mult = {"core": 1.7, "handler": 1.4, "job": 1.2, "report": 1.3, "service": 1.0, "domain": 1.1, "lib": 0.8}.get(role, 1.0)
        return synth_clj(ns, lognorm(115 * mult, 0.75, 10, 1900),
                         lambda: weighted([("defn", .58), ("defn-", .27), ("def", .08), ("defmulti", .02), ("defmethod", .04), ("defmacro", .01)]),
                         lambda: fname(key))
    ext = name.rsplit(".", 1)[-1] if "." in name else ""
    if ext == "feature":
        return synth_blocks(lognorm(36, 0.5, 10, 160), "Scenario", lambda: f"{rng.choice(VERBS)} a {rng.choice(THINGS)}",
                            body_ind=4, deep=2)
    if lang == "sql-mig":
        return synth_sql(lognorm(22, 0.6, 5, 120), "mig")
    if lang in ("sql-q", "sql-model"):
        return synth_sql(lognorm(90 if lang == "sql-q" else 75, 0.6, 12, 600), "model")
    if ext in ("yml", "yaml"):
        return synth_nested(lognorm(40, 0.6, 8, 400), maxd=5, keyout=lambda: rng.choice(["name", "on", "jobs", "env", "models", "sources", "version", "spec", "metadata", "apiVersion", "kind", "data", "steps", "services", "columns", "tests"]))
    if ext == "tf" or ext == "tfvars":
        return synth_blocks(lognorm(45, 0.6, 8, 400), "resource", lambda: f"{rng.choice(['aws_s3_bucket', 'aws_db_instance', 'aws_iam_role', 'aws_sqs_queue', 'aws_security_group', 'aws_route53_record', 'aws_cloudwatch_metric_alarm', 'aws_elasticache_cluster', 'variable', 'output', 'module'])}.{rng.choice(DOMAINS + ['main', 'primary', 'archive', 'logs'])}")
    if ext in ("html", "xml"):
        return synth_nested(lognorm(70, 0.6, 12, 500), maxd=8, lo=8, hi=64)
    if ext == "css":
        return synth_blocks(lognorm(130, 0.7, 20, 1200), "rule", lambda: "." + rng.choice(COMPONENTS).replace("_", "-"), deep=2)
    if ext == "json":
        return synth_nested(lognorm(45, 0.5, 8, 200), maxd=4, lo=8, hi=50)
    if ext == "edn":
        return synth_nested(lognorm(40, 0.8, 4, 900), maxd=5, lo=6, hi=58, keyout=lambda: ":" + rng.choice(THINGS + DOMAINS))
    if ext == "py":
        return synth_blocks(lognorm(130, 0.6, 20, 900), rng.choice(["def", "class"]), lambda: f"{rng.choice(VERBS)}_{rng.choice(THINGS).replace('-', '_')}", body_ind=4, deep=8)
    if ext in ("sh",):
        return synth_nested(lognorm(45, 0.6, 8, 300), maxd=3, lo=8, hi=78)
    if ext in ("js",):
        return synth_nested(lognorm(30, 0.4, 8, 90), maxd=4, lo=6, hi=50)
    if ext == "csv":
        n = lognorm(80, 0.9, 5, 3000)
        return [(0, rng.randint(30, 70)) for _ in range(n)], []
    return synth_nested(lognorm(30, 0.6, 3, 200), maxd=3, lo=4, hi=60)


def minimap(L):
    n = len(L)
    R = min(n, 64)
    out = []
    for r in range(R):
        a, b = r * n // R, max((r + 1) * n // R, r * n // R + 1)
        ind, ln = 999, 0
        for (i, l) in L[a:b]:
            if l <= 0:
                continue
            ind = min(ind, i); ln = max(ln, min(l, 126))
        if ln == 0:
            ind = 0
        out.append(B64[min(ind // 2, 63)] + B64[min((ln + 1) // 2, 63)])
    return "".join(out)


# ---------------- git ----------------
AGE_MAX = 2560
ERA = {"components/edifact": (700, 2300), "bases/api": (1200, 2550), "bases/worker": (400, 2200),
       "bases/reporting": (200, 1800), "migrations": None, "bases/console": (500, 2200), "bases/portal": (250, 1300),
       "bases/mobile": (60, 650), "components/ui-kit": (400, 2300), "development/cards": (150, 1200),
       "insights": (300, 1500), "bases/admin": (100, 1500), "acceptance": (200, 1400), "deploy": (300, 2400),
       "docs/decisions": None, "components/specs": (600, 2400), "components/units": (600, 2400)}
ERA.update({f"components/{d}/": (1400, 2550) for d in DOMAINS})
ERA.update({f"components/{x.replace('_', '-')}/": (650, 2300) for x in SERVICES + INTEGRATIONS})
HOT = ["berth", "booking", "invoice", "gate", "appointment", "driver", "portal", "planner", "eta", "tariff"]
commits = {}
pool = defaultdict(list)
TYPES = [("feat", .34), ("fix", .30), ("chore", .14), ("test", .08), ("refactor", .08), ("docs", .06)]
TEMPL = {
    "feat": ["Add {k} {t} {x}", "Support {k} {t} in {where}", "Show {k} {t} in {where}", "Allow bulk {k} {t} updates", "Add {k} {t} export"],
    "fix": ["Handle missing {t} in {k} sync", "Correct {k} {t} rounding", "Prevent duplicate {k} {t}", "Fix {k} {t} timezone drift", "Stop {k} {t} retry loop"],
    "chore": ["Rename {k} {t} keys", "Extract {k} {t} helper", "Remove unused {k} {t} code", "Tidy {k} {t} logging"],
    "test": ["Cover {k} {t} edge cases", "Add {k} {t} regression test", "Speed up {k} {t} tests"],
    "refactor": ["Split {k} {t} namespace", "Move {k} {t} into its component", "Simplify {k} {t} flow"],
    "docs": ["Document {k} {t}", "Update {k} playbook", "Add a decision on {k} {t}"],
}
WHERE = ["console", "portal", "mobile app", "API", "nightly job", "reports", "admin CLI"]
LIBS = ["reitit", "integrant", "honeysql", "next.jdbc", "re-frame", "shadow-cljs", "malli", "clj-kondo", "tick", "etaoin"]


def subject(k, age):
    """A subject in the area-prefix style, such as `berth: Add window conflict check`."""
    t = weighted(TYPES)
    if t == "chore" and rng.random() < 0.25:
        lib = rng.choice(LIBS)
        return f"deps: Update {lib} to {rng.randint(0, 3)}.{rng.randint(1, 40)}.{rng.randint(0, 9)}"
    s = rng.choice(TEMPL[t]).format(k=words(k), t=rng.choice(THINGS).replace("-", " "), x=rng.choice(["check", "view", "filter", "limit", "flag", "column"]), where=rng.choice(WHERE))
    return f"{k.replace('_', '-')}: {s[0].upper() + s[1:]}"


def commit_for(k, age):
    b = age // 6
    for c in pool[(k, b)]:
        return c
    h = "%09x" % rng.getrandbits(36)
    commits[h] = [h, age, subject(k, age)]
    pool[(k, b)].append(h)
    return h


def key_of(f):
    k = f.get("key")
    if k and "/" not in k:
        return k
    parts = f["p"].split("/")
    for part in parts:
        base = part.rsplit(".", 1)[0]
        for vocab in (DOMAINS, SERVICES, INTEGRATIONS):
            if base in vocab:
                return base
    return parts[0] if len(parts) > 1 else "repo"


def git_for(f):
    p = f["p"]
    era = (100, AGE_MAX)
    for pre, e in ERA.items():
        if p.startswith(pre):
            era = e
            break
    if "mig" in f and p.startswith("migrations/"):
        a0 = int(AGE_MAX - f["mig"] * (AGE_MAX - 30) / MIGS); a0 = max(20, a0)
    elif "mig" in f:
        a0 = max(10, int(1500 - f["mig"] * 45))
    elif "adr" in f:
        a0 = max(15, int(AGE_MAX - f["adr"] * 60))
    elif era is None:
        a0 = rng.randint(100, AGE_MAX)
    else:
        a0 = rng.randint(*era)
    hot = any(h in p for h in HOT)
    if "migrations" in p or "adr" in p:
        a1 = int(a0 * rng.random() ** 0.2)
    else:
        a1 = int(a0 * rng.random() ** (3.2 if hot else 1.4))
    n = f.get("n") or 20
    c = max(1, int(math.exp(rng.gauss(math.log(1.5 + n / 70 + a0 / 365 * 1.4 * (2 if hot else 1)), 0.55))))
    if "migrations" in p:
        c = rng.randint(1, 2)
    if a1 == a0:
        c = 1
    c = min(c, 420)
    k = key_of(f)
    ages = [a1]
    if c >= 2:
        ages.append(a0 if c == 2 else rng.randint(a1, a0))
    if c >= 3:
        ages.append(rng.randint(ages[-1], a0))
    ages = sorted(set(ages))
    rs = [commit_for(k, a) for a in ages]
    f["a1"] = commits[rs[0]][1]
    f["a0"] = max(a0, commits[rs[-1]][1])
    f["c"] = max(c, len(rs))
    f["r"] = rs


# ---------------- build ----------------
PINS = {"components/berth/src/quayside/berth/rules.clj": (9, "window-conflicts?"),
        "bases/worker/src/quayside/worker/jobs/berth_planner.clj": (6, "plan-voyage-windows"),
        "bases/console/src/quayside/console/views/berth/planner.cljs": (7, "on-window-drop"),
        "components/berth/src/quayside/berth/allocation.clj": (4, "allocate-window!")}
out = []
for f in files:
    rec = {"p": f["p"]}
    FORCE[0] = MIN_N.get(f["p"])
    L = lines_for(f)
    if L is None:
        rec["b"] = 1
        rec["sz"] = rng.randint(400, 60000)
        f["n"] = 0
    else:
        lines, O = L
        rec["n"] = len(lines); f["n"] = len(lines)
        rec["m"] = minimap(lines)
        if O:
            rec["o"] = O[:160]
    pin = PINS.get(f["p"])
    if pin and rec.get("o"):
        idx = min(pin[0], len(rec["o"]) - 1)
        rec["o"][idx][1] = "defn"
        rec["o"][idx][2] = pin[1]
        print("PIN", f["p"], rec["o"][idx][0], pin[1])
    ns = ns_of(f["p"]) if f["p"].rsplit(".", 1)[-1] in ("clj", "cljs", "cljc") else None
    if ns:
        rec["ns"] = ns
    if f.get("ig"):
        rec["ig"] = 1
    git_for(f)
    for k in ("a1", "a0", "c", "r"):
        rec[k] = f[k]
    out.append(rec)

used = {h for r in out for h in r["r"]}
data = {"root": "quayside", "files": out, "commits": {h: commits[h] for h in used}}
json.dump(data, open(sys.argv[1], "w"), separators=(",", ":"))

# report size and compacted directory count (same rule as the page)
dirs = set()
fcount = defaultdict(int)
for r in out:
    parts = r["p"].split("/")
    fcount["/".join(parts[:-1])] += 1
    for i in range(1, len(parts)):
        dirs.add("/".join(parts[:i]))
kids = defaultdict(set)
for d in dirs:
    kids[d.rsplit("/", 1)[0] if "/" in d else ""].add(d)
compacted = sum(1 for d in dirs if fcount[d] == 0 and len(kids[d]) == 1)
print(len(out), "files;", len(dirs) - compacted + 1, "directory nodes;", len(data["commits"]), "commits;",
      os.path.getsize(sys.argv[1]), "bytes;", sum(r.get("n", 0) for r in out), "lines")
