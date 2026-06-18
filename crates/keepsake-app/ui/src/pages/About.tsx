import { state } from "../state";

const REPO_URL = "https://github.com/JustinWoodring/keepsake";

export function AboutPage() {
  return (
    <div class="page">
      <header class="page-header">
        <div>
          <h1 class="page-title">
            <span class="about-mark" aria-hidden="true">⛨</span> About
          </h1>
          <p class="page-sub">
            End-to-end-encrypted life organizer.  Local-first,
            sync-optional, open source.
          </p>
        </div>
      </header>

      <div class="settings-section">
        <div class="about-row">
          <span class="about-icon" aria-hidden="true">⚙</span>
          <div>
            <div class="about-label">version</div>
            <div class="about-value">0.1.2</div>
          </div>
        </div>

        <div class="about-row">
          <span class="about-icon" aria-hidden="true">📄</span>
          <div>
            <div class="about-label">license</div>
            <div class="about-value">MIT</div>
          </div>
        </div>

        <div class="about-row">
          <span class="about-icon" aria-hidden="true">🔐</span>
          <div>
            <div class="about-label">crypto</div>
            <div class="about-value">
              XChaCha20-Poly1305 (AEAD), Argon2id (KDF),
              BLAKE3 (hashing), nested vault key + shared
              sync key envelopes
            </div>
          </div>
        </div>
      </div>

      <div class="settings-section">
        <h2 class="settings-section-title">Project</h2>
        <a
          class="btn btn-block btn-lg about-link"
          href={REPO_URL}
          target="_blank"
          rel="noreferrer noopener"
        >
          <span class="about-external" aria-hidden="true">↗</span>
          <span>github.com/JustinWoodring/keepsake</span>
        </a>

        <div class="about-row">
          <span class="about-icon" aria-hidden="true">📚</span>
          <div>
            <div class="about-label">documentation</div>
            <div class="about-value">
              <a href={`${REPO_URL}/tree/main/docs`} target="_blank" rel="noreferrer noopener">
                docs/
              </a>{" "}
              covers the{" "}
              <a href={`${REPO_URL}/blob/main/docs/sync-protocol.md`} target="_blank" rel="noreferrer noopener">
                sync protocol
              </a>,{" "}
              <a href={`${REPO_URL}/blob/main/docs/crypto.md`} target="_blank" rel="noreferrer noopener">
                crypto
              </a>, and the{" "}
              <a href={`${REPO_URL}/blob/main/docs/threat-model.md`} target="_blank" rel="noreferrer noopener">
                threat model
              </a>.
            </div>
          </div>
        </div>

        <div class="about-row">
          <span class="about-icon" aria-hidden="true">🐛</span>
          <div>
            <div class="about-label">issues &amp; feature requests</div>
            <div class="about-value">
              <a
                href={`${REPO_URL}/issues`}
                target="_blank"
                rel="noreferrer noopener"
              >
                Open an issue
              </a>{" "}
              on GitHub.
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
