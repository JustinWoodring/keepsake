/* @refresh reload */
import { render } from "solid-js/web";
import { Router, Route } from "@solidjs/router";
import { App } from "./App";
import { Dashboard } from "./pages/Dashboard";
import { Category } from "./pages/Category";
import { RecordDetail } from "./pages/RecordDetail";
import { RecordForm } from "./pages/RecordForm";
import { AuditPage } from "./pages/Audit";
import { Settings } from "./pages/Settings";
import { SyncPage } from "./pages/Sync";
import { Insights } from "./pages/Insights";
import { AboutPage } from "./pages/About";
import "./styles.css";

const root = document.getElementById("root");
if (root) {
  render(
    () => (
      <Router root={App}>
        <Route path="/" component={Dashboard} />
        <Route path="/c/:type" component={Category} />
        <Route path="/r/:id" component={RecordDetail} />
        <Route path="/c/:type/new" component={RecordForm} />
        <Route path="/r/:id/edit" component={RecordForm} />
        <Route path="/audit" component={AuditPage} />
        <Route path="/sync" component={SyncPage} />
        <Route path="/insights" component={Insights} />
        <Route path="/settings" component={Settings} />
        <Route path="/about" component={AboutPage} />
      </Router>
    ),
    root,
  );
}
