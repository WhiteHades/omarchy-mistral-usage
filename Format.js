.pragma library

function money(value, currency) {
    return Number(value).toLocaleCurrencyString(Qt.locale(), currency === "USD" ? "$" : currency + " ")
}

function resetTime(budget, now) {
    var stamp = Date.parse(budget.resetAt || "")
    if (isFinite(stamp)) return stamp
    // The API can omit a reset. The monthly boundary is UTC, never local time.
    var date = new Date(now)
    return Date.UTC(date.getUTCFullYear(), date.getUTCMonth() + 1, 1)
}
