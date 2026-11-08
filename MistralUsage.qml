import QtQuick
import Quickshell
import Quickshell.Io
import "Format.js" as Format

Item {
  id: root
  visible: false
  property var report: ({status: "loading", budgets: []})
  readonly property bool busy: fetcher.running

  function refresh() { if (enabled && !busy) fetcher.running = true }
  function refreshIfExpired(now) {
    var observed = Number(report.checkedAt || 0) * 1000
    if (report.status !== "ready" || now - observed < 60000) return
    for (var i = 0; i < report.budgets.length; i++) {
      if (Format.resetTime(report.budgets[i], observed) <= now) {
        refresh()
        break
      }
    }
  }
  onEnabledChanged: {
    if (enabled) refresh()
    else {
      fetcher.running = false
      report = {status: "disabled", budgets: []}
    }
  }
  Component.onDestruction: fetcher.running = false

  Process {
    id: fetcher
    command: [decodeURIComponent(Qt.resolvedUrl("bin/omarchy-mistral-usage").toString().replace(/^file:\/\//, ""))]
    stdout: StdioCollector {
      onStreamFinished: {
        try {
          var data = JSON.parse(text)
          if (!data || !Array.isArray(data.budgets) || typeof data.status !== "string") throw new Error("Invalid report")
          root.report = data
        } catch (error) { root.report = {status: "unavailable", budgets: []} }
      }
    }
    onExited: function(exitCode, exitStatus) {
      if (exitCode !== 0) root.report = {status: "unavailable", budgets: []}
    }
  }
}
