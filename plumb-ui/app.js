// ─── Config ────────────────────────────────────────────────────────────────
const API_BASE = '/api';
const NODE_W = 180;
const NODE_H = 80;
const SVG_NS = 'http://www.w3.org/2000/svg';

// ─── Global state ──────────────────────────────────────────────────────────
let pipelines = [];
let filteredPipelines = [];
let selectedPipeline = null;
let availableConnectors = [];

// ─── Canvas state ──────────────────────────────────────────────────────────
let nodePositions = {};        // { [nodeId]: { x, y } }
let currentPipelineId = null;
let viewTranslate = { x: 60, y: 60 };
let viewScale = 1;
let isPanning = false;
let panStart = { x: 0, y: 0 };
let draggingNode = null;
let dragOffset = { x: 0, y: 0 };
let connectingFrom = null;     // { nodeId, x, y } in canvas coords

// ─── Add-node modal state ──────────────────────────────────────────────────
let nodeModalStep = 1;
let selectedCategory = null;   // 'connector' | 'transformation' | 'destination'
let selectedConnType = null;
let selectedConnMeta = null;   // full connector object from /api/connectors

// ─── DOM helpers ───────────────────────────────────────────────────────────
const el = id => document.getElementById(id);
const svgEl = tag => document.createElementNS(SVG_NS, tag);

const elements = {
    get pipelineList()   { return el('pipeline-list'); },
    get canvasTitle()    { return el('canvas-title'); },
    get canvasSubtitle() { return el('canvas-subtitle'); },
    get pipelineCanvas() { return el('pipeline-canvas'); },
    get emptyState()     { return el('empty-state'); },
    get modal()          { return el('new-pipeline-modal'); },
    get pipelineForm()   { return el('new-pipeline-form'); },
    get pipelineNameInput() { return el('pipeline-name'); },
    get searchInput()    { return el('pipeline-search'); },
};

// ─── Position persistence ──────────────────────────────────────────────────
function posKey(pipelineId) { return `plumb_pos_${pipelineId}`; }

function savePositions(pipelineId) {
    localStorage.setItem(posKey(pipelineId), JSON.stringify(nodePositions));
}

function loadPositions(pipelineId) {
    try {
        const raw = localStorage.getItem(posKey(pipelineId));
        return raw ? JSON.parse(raw) : {};
    } catch { return {}; }
}

// ─── Colour helpers ────────────────────────────────────────────────────────
function nodeColor(kind) {
    return { connector: '#3b82f6', transformation: '#8b5cf6', destination: '#f59e0b' }[kind] ?? '#6b7280';
}

function bezierPath(x1, y1, x2, y2) {
    const dx = Math.max(Math.abs(x2 - x1) * 0.55, 80);
    return `M ${x1} ${y1} C ${x1 + dx} ${y1} ${x2 - dx} ${y2} ${x2} ${y2}`;
}

// Convert mouse event to SVG canvas coordinates (accounts for pan + zoom)
function svgPoint(e) {
    const rect = elements.pipelineCanvas.getBoundingClientRect();
    return {
        x: (e.clientX - rect.left - viewTranslate.x) / viewScale,
        y: (e.clientY - rect.top  - viewTranslate.y) / viewScale,
    };
}

function updateViewport() {
    const vp = el('canvas-viewport');
    if (vp) vp.setAttribute('transform',
        `translate(${viewTranslate.x},${viewTranslate.y}) scale(${viewScale})`);
}

// ─── API ───────────────────────────────────────────────────────────────────
async function fetchConnectors() {
    try {
        const res = await fetch(`${API_BASE}/connectors`);
        if (!res.ok) throw new Error();
        availableConnectors = await res.json();
        renderConnectorTypeGrid();
    } catch {
        availableConnectors = [];
    }
}

async function createPipeline(name) {
    const res = await fetch(`${API_BASE}/pipelines`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ id: 0, name, nodes: [], edges: [] }),
    });
    if (!res.ok) throw new Error(`HTTP ${res.status}`);
    return res.json();
}

async function apiCreateEdge(pipelineId, from, to) {
    const res = await fetch(`${API_BASE}/pipelines/${pipelineId}/edges`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ from, to }),
    });
    if (!res.ok) throw new Error(`HTTP ${res.status}`);
}

async function loadPipelines() {
    try {
        const res = await fetch(`${API_BASE}/pipelines`);
        if (!res.ok) throw new Error(`HTTP ${res.status}`);
        pipelines = await res.json();
        filteredPipelines = [...pipelines];
        renderPipelineList();
        updateStatusSummary();
        if (pipelines.length > 0 && !selectedPipeline) selectPipeline(pipelines[0].id);
    } catch {
        showNotification('Failed to load pipelines', 'error');
        if (elements.pipelineList) {
            elements.pipelineList.innerHTML = `
                <div class="empty-list">
                    <p>Failed to load pipelines</p>
                    <button class="btn btn-secondary" onclick="loadPipelines()">Retry</button>
                </div>`;
        }
    }
}

// ─── Pipeline list UI ──────────────────────────────────────────────────────
function renderPipelineList() {
    const list = elements.pipelineList;
    if (!list) return;

    if (!filteredPipelines.length) {
        const searching = elements.searchInput?.value.trim() !== '';
        list.innerHTML = `
            <div class="empty-list">
                <p>${searching ? 'No results' : 'No pipelines'}</p>
                ${!searching ? '<button class="btn btn-primary" onclick="openCreateModal()">Create First Pipeline</button>' : ''}
            </div>`;
        return;
    }

    list.innerHTML = filteredPipelines.map(p => `
        <div class="pipeline-item ${selectedPipeline?.id === p.id ? 'active' : ''}" data-id="${p.id}"
             onclick="selectPipeline(${p.id})">
            <div class="pipeline-name" title="${p.name}">${p.name}</div>
            <div class="pipeline-meta">
                <div class="pipeline-status ${p.status ?? 'idle'}"></div>
                <span>${p.nodes?.length ?? 0}</span>
            </div>
        </div>`).join('');
}

function updateStatusSummary() {
    let running = 0, idle = 0, error = 0;
    filteredPipelines.forEach(p => {
        if (p.status === 'running') running++;
        else if (p.status === 'error') error++;
        else idle++;
    });
    const r = el('running-count'), i = el('idle-count'), e = el('error-count');
    if (r) r.textContent = running;
    if (i) i.textContent = idle;
    if (e) e.textContent = error;
}

function selectPipeline(id) {
    const pipeline = pipelines.find(p => p.id === id);
    if (!pipeline) return;

    // Reset canvas view when switching pipelines
    if (currentPipelineId !== id) {
        currentPipelineId = id;
        nodePositions = loadPositions(id);
        viewTranslate = { x: 60, y: 60 };
        viewScale = 1;
    }

    selectedPipeline = pipeline;

    document.querySelectorAll('.pipeline-item').forEach(e => e.classList.remove('active'));
    document.querySelector(`.pipeline-item[data-id="${id}"]`)?.classList.add('active');

    renderPipelineCanvas(pipeline);
    elements.canvasTitle.textContent = pipeline.name;
    elements.canvasSubtitle.textContent =
        `${pipeline.nodes?.length ?? 0} nodes · ${pipeline.edges?.length ?? 0} connections`;
    elements.emptyState.style.display = 'none';
    el('add-node-btn').disabled = false;
    el('validate-btn').disabled = false;
    el('run-btn').disabled = false;
}

// ─── Canvas rendering ──────────────────────────────────────────────────────
function ensurePositions(nodes) {
    const unpositioned = nodes.filter(n => !nodePositions[n.id]);
    if (!unpositioned.length) return;

    // Column x positions by node kind
    const colX   = { connector: 80, transformation: 320, destination: 560 };
    // Track next available y per column (initialise below any already-placed nodes)
    const nextY  = { connector: 60, transformation: 60, destination: 60 };
    nodes.forEach(n => {
        if (nodePositions[n.id]) {
            const k = n.node_type?.kind ?? 'destination';
            const col = colX[k] !== undefined ? k : 'destination';
            nextY[col] = Math.max(nextY[col], nodePositions[n.id].y + NODE_H + 40);
        }
    });

    unpositioned.forEach(n => {
        const k = n.node_type?.kind ?? 'destination';
        const col = colX[k] !== undefined ? k : 'destination';
        nodePositions[n.id] = { x: colX[col], y: nextY[col] };
        nextY[col] += NODE_H + 40;
    });
}

function buildNodeElement(node) {
    const { x, y } = nodePositions[node.id];
    const kind  = node.node_type?.kind ?? 'destination';
    const color = nodeColor(kind);
    const W = NODE_W, H = NODE_H;

    const g = svgEl('g');
    g.setAttribute('class', 'canvas-node');
    g.setAttribute('transform', `translate(${x},${y})`);
    g.setAttribute('data-node-id', node.id);

    // Drop shadow
    const shadow = svgEl('rect');
    shadow.setAttribute('x', '4');   shadow.setAttribute('y', '5');
    shadow.setAttribute('width', W); shadow.setAttribute('height', H);
    shadow.setAttribute('rx', '10');
    shadow.setAttribute('fill', 'rgba(0,0,0,0.45)');

    // Card body
    const body = svgEl('rect');
    body.setAttribute('width', W); body.setAttribute('height', H);
    body.setAttribute('rx', '10');
    body.setAttribute('fill', '#0d1929');
    body.setAttribute('stroke', color);
    body.setAttribute('stroke-width', '1.5');
    body.setAttribute('stroke-opacity', '0.5');

    // Left colour accent bar
    const accent = svgEl('rect');
    accent.setAttribute('x', '0');  accent.setAttribute('y', '16');
    accent.setAttribute('width', '4'); accent.setAttribute('height', H - 32);
    accent.setAttribute('rx', '2');
    accent.setAttribute('fill', color);

    // Type label (small caps)
    const typeLabel = svgEl('text');
    typeLabel.setAttribute('x', '14'); typeLabel.setAttribute('y', '28');
    typeLabel.setAttribute('font-size', '9');
    typeLabel.setAttribute('font-weight', '700');
    typeLabel.setAttribute('letter-spacing', '1.2');
    typeLabel.setAttribute('fill', color);
    typeLabel.textContent = kind.toUpperCase();

    // Node name
    const nameLabel = svgEl('text');
    nameLabel.setAttribute('x', '14'); nameLabel.setAttribute('y', '56');
    nameLabel.setAttribute('font-size', '13');
    nameLabel.setAttribute('font-weight', '600');
    nameLabel.setAttribute('fill', '#f1f5f9');
    nameLabel.textContent = node.name.length > 17 ? node.name.slice(0, 16) + '…' : node.name;

    // Input port — left mid-edge
    const inPort = svgEl('circle');
    inPort.setAttribute('class', 'port port-input');
    inPort.setAttribute('cx', '0');   inPort.setAttribute('cy', H / 2);
    inPort.setAttribute('r', '7');
    inPort.setAttribute('fill', '#0d1929');
    inPort.setAttribute('stroke', color);
    inPort.setAttribute('stroke-width', '2');
    inPort.setAttribute('data-node-id', node.id);

    // Output port — right mid-edge
    const outPort = svgEl('circle');
    outPort.setAttribute('class', 'port port-output');
    outPort.setAttribute('cx', W);    outPort.setAttribute('cy', H / 2);
    outPort.setAttribute('r', '7');
    outPort.setAttribute('fill', color);
    outPort.setAttribute('stroke', '#f8fafc');
    outPort.setAttribute('stroke-width', '2');
    outPort.setAttribute('data-node-id', node.id);

    // Transparent drag-handle overlay (sits above body, below ports conceptually
    // but ports get events first because they are rendered later in the DOM)
    const handle = svgEl('rect');
    handle.setAttribute('class', 'node-drag-handle');
    handle.setAttribute('x', '8');   handle.setAttribute('y', '0');
    handle.setAttribute('width', W - 16); handle.setAttribute('height', H);
    handle.setAttribute('rx', '10');
    handle.setAttribute('fill', 'transparent');
    handle.setAttribute('data-node-id', node.id);

    g.append(shadow, body, accent, typeLabel, nameLabel, handle, inPort, outPort);
    return g;
}

function refreshEdges() {
    const layer = el('edges-layer');
    if (!layer || !selectedPipeline) return;
    layer.innerHTML = '';
    (selectedPipeline.edges ?? []).forEach(edge => {
        const from = nodePositions[edge.from];
        const to   = nodePositions[edge.to];
        if (!from || !to) return;
        const path = svgEl('path');
        path.setAttribute('d', bezierPath(
            from.x + NODE_W, from.y + NODE_H / 2,
            to.x,            to.y   + NODE_H / 2,
        ));
        path.setAttribute('stroke', '#475569');
        path.setAttribute('stroke-width', '2');
        path.setAttribute('fill', 'none');
        path.setAttribute('marker-end', 'url(#arrow)');
        layer.appendChild(path);
    });
}

function renderPipelineCanvas(pipeline) {
    const svg = elements.pipelineCanvas;
    svg.innerHTML = '';

    // Arrow marker
    const defs = svgEl('defs');
    defs.innerHTML = `
        <marker id="arrow" markerWidth="8" markerHeight="6" refX="7" refY="3" orient="auto">
            <polygon points="0 0, 8 3, 0 6" fill="#475569"/>
        </marker>`;

    // Viewport group — receives pan/zoom transform
    const viewport = svgEl('g');
    viewport.id = 'canvas-viewport';
    viewport.setAttribute('transform',
        `translate(${viewTranslate.x},${viewTranslate.y}) scale(${viewScale})`);

    const edgesLayer = svgEl('g'); edgesLayer.id = 'edges-layer';
    const nodesLayer = svgEl('g'); nodesLayer.id = 'nodes-layer';

    // Draft line shown while drawing a connection
    const draft = svgEl('path');
    draft.id = 'connection-draft';
    draft.setAttribute('stroke', '#3b82f6');
    draft.setAttribute('stroke-width', '2');
    draft.setAttribute('stroke-dasharray', '6 4');
    draft.setAttribute('fill', 'none');
    draft.setAttribute('pointer-events', 'none');
    draft.style.display = 'none';

    viewport.append(edgesLayer, nodesLayer, draft);
    svg.append(defs, viewport);

    if (!pipeline.nodes?.length) return;

    ensurePositions(pipeline.nodes);
    pipeline.nodes.forEach(node => nodesLayer.appendChild(buildNodeElement(node)));
    refreshEdges();
}

// ─── Canvas interactions ───────────────────────────────────────────────────
function initCanvasEvents() {
    const svg     = elements.pipelineCanvas;
    const wrapper = document.querySelector('.canvas-wrapper');

    // ── Zoom via scroll wheel ──────────────────────────────────────────
    svg.addEventListener('wheel', e => {
        e.preventDefault();
        const factor = e.deltaY < 0 ? 1.1 : 0.9;
        const rect = svg.getBoundingClientRect();
        const mx = e.clientX - rect.left;
        const my = e.clientY - rect.top;
        viewTranslate.x = mx - (mx - viewTranslate.x) * factor;
        viewTranslate.y = my - (my - viewTranslate.y) * factor;
        viewScale = Math.min(Math.max(viewScale * factor, 0.15), 5);
        updateViewport();
    }, { passive: false });

    // ── Mousedown: start pan / drag / connect ─────────────────────────
    svg.addEventListener('mousedown', e => {
        const t = e.target;

        if (t.classList.contains('port-output')) {
            e.stopPropagation();
            const nodeId = parseInt(t.getAttribute('data-node-id'));
            const pos = nodePositions[nodeId];
            if (!pos) return;
            connectingFrom = { nodeId, x: pos.x + NODE_W, y: pos.y + NODE_H / 2 };
            const draft = el('connection-draft');
            if (draft) draft.style.display = '';
            wrapper.style.cursor = 'crosshair';
            return;
        }

        if (t.classList.contains('node-drag-handle')) {
            e.stopPropagation();
            const nodeId = parseInt(t.getAttribute('data-node-id'));
            const pos = nodePositions[nodeId];
            if (!pos) return;
            const sp = svgPoint(e);
            draggingNode = nodeId;
            dragOffset   = { x: sp.x - pos.x, y: sp.y - pos.y };
            wrapper.style.cursor = 'grabbing';
            return;
        }

        // Anything else: pan
        if (!t.classList.contains('port-input')) {
            isPanning = true;
            panStart  = { x: e.clientX - viewTranslate.x, y: e.clientY - viewTranslate.y };
            wrapper.style.cursor = 'grabbing';
        }
    });

    // ── Mousemove ─────────────────────────────────────────────────────
    document.addEventListener('mousemove', e => {
        if (draggingNode !== null) {
            const sp = svgPoint(e);
            nodePositions[draggingNode] = { x: sp.x - dragOffset.x, y: sp.y - dragOffset.y };
            const nodeEl = svg.querySelector(`.canvas-node[data-node-id="${draggingNode}"]`);
            if (nodeEl) {
                const { x, y } = nodePositions[draggingNode];
                nodeEl.setAttribute('transform', `translate(${x},${y})`);
            }
            refreshEdges();
            return;
        }

        if (connectingFrom) {
            const sp    = svgPoint(e);
            const draft = el('connection-draft');
            if (draft) draft.setAttribute('d',
                bezierPath(connectingFrom.x, connectingFrom.y, sp.x, sp.y));
            return;
        }

        if (isPanning) {
            viewTranslate.x = e.clientX - panStart.x;
            viewTranslate.y = e.clientY - panStart.y;
            updateViewport();
        }
    });

    // ── Mouseup ───────────────────────────────────────────────────────
    document.addEventListener('mouseup', e => {
        if (connectingFrom) {
            const t = e.target;
            if (t.classList.contains('port-input')) {
                const toId = parseInt(t.getAttribute('data-node-id'));
                if (toId !== connectingFrom.nodeId) finalizeConnection(connectingFrom.nodeId, toId);
            }
            connectingFrom = null;
            const draft = el('connection-draft');
            if (draft) { draft.style.display = 'none'; draft.setAttribute('d', ''); }
            wrapper.style.cursor = '';
        }
        if (draggingNode !== null) {
            draggingNode = null;
            wrapper.style.cursor = '';
            if (currentPipelineId) savePositions(currentPipelineId);
        }
        if (isPanning)             { isPanning = false;   wrapper.style.cursor = ''; }
    });
}

async function finalizeConnection(fromId, toId) {
    if (!selectedPipeline) return;
    if (selectedPipeline.edges?.some(e => e.from === fromId && e.to === toId)) return;
    try {
        await apiCreateEdge(selectedPipeline.id, fromId, toId);
        if (!selectedPipeline.edges) selectedPipeline.edges = [];
        selectedPipeline.edges.push({ from: fromId, to: toId });
        refreshEdges();
        showNotification('Connected', 'success');
    } catch {
        showNotification('Failed to connect nodes', 'error');
    }
}

// ─── Add Node modal ────────────────────────────────────────────────────────
function openAddNodeModal() {
    nodeModalStep    = 1;
    selectedCategory = null;
    selectedConnType = null;
    goToStep(1);
    el('add-node-modal').classList.add('active');
}

function closeAddNodeModal() {
    el('add-node-modal').classList.remove('active');
    el('add-node-form')?.reset();
    el('node-config-fields').innerHTML = '';
    selectedConnMeta = null;
}

function goToStep(step) {
    nodeModalStep = step;
    [1, 2, 3].forEach(s => {
        const stepEl = el(`node-step-${s}`);
        if (stepEl) stepEl.style.display = s === step ? '' : 'none';
    });
    el('node-modal-back').style.display    = step > 1 ? '' : 'none';
    el('confirm-add-node').style.display   = step === 3 ? '' : 'none';
    el('node-modal-title').textContent     =
        { 1: 'Add Node', 2: 'Choose Connector', 3: 'Configure Node' }[step] ?? 'Add Node';
}

function goNodeModalBack() {
    goToStep(nodeModalStep === 3 && selectedCategory === 'connector' ? 2 : 1);
}

function handleCategorySelect(category) {
    selectedCategory = category;
    selectedConnMeta = null;

    if (category === 'connector') {
        goToStep(2);
    } else {
        el('node-name').value = category === 'transformation' ? 'Transform' : 'Output';
        renderSchemaForm(null);
        goToStep(3);
    }
}

function handleConnectorTypeSelect(type, displayName) {
    selectedConnType = type;
    selectedConnMeta = availableConnectors.find(c => c.type === type) ?? null;
    el('node-name').value = displayName;
    renderSchemaForm(selectedConnMeta?.config_schema ?? null);
    goToStep(3);
}

// ─── Schema-driven form renderer ──────────────────────────────────────────
function renderSchemaForm(schema) {
    const container = el('node-config-fields');
    container.innerHTML = '';
    if (!schema?.properties) return;

    const required = new Set(schema.required ?? []);

    Object.entries(schema.properties).forEach(([key, prop]) => {
        const isRequired = required.has(key);
        const label = key.replace(/_/g, ' ').replace(/\b\w/g, c => c.toUpperCase());
        const desc  = prop.description ?? '';
        const type  = prop.type ?? 'string';

        const group = document.createElement('div');
        group.className = 'form-group';

        const labelEl = document.createElement('label');
        labelEl.setAttribute('for', `cfg-${key}`);
        labelEl.innerHTML = label + (isRequired ? ' <span class="field-required">*</span>' : '');

        let input;

        if (type === 'boolean') {
            const wrapper = document.createElement('div');
            wrapper.className = 'toggle-wrapper';
            input = document.createElement('input');
            input.type = 'checkbox';
            input.id = `cfg-${key}`;
            input.name = key;
            input.className = 'toggle-input';
            input.checked = prop.default ?? false;
            const toggleTrack = document.createElement('label');
            toggleTrack.setAttribute('for', `cfg-${key}`);
            toggleTrack.className = 'toggle-track';
            wrapper.append(input, toggleTrack);
            if (desc) {
                const hint = document.createElement('span');
                hint.className = 'field-hint';
                hint.textContent = desc;
                wrapper.append(hint);
            }
            group.append(labelEl, wrapper);
        } else if (type === 'number' || type === 'integer') {
            input = document.createElement('input');
            input.type = 'number';
            input.id = `cfg-${key}`;
            input.name = key;
            if (prop.default !== undefined) input.value = prop.default;
            if (isRequired) input.required = true;
            if (desc) input.placeholder = desc;
            if (desc) addHint(group, desc);
            group.append(labelEl, input);
        } else if (key === 'file_path' || key.endsWith('_path') || key.endsWith('_file')) {
            // File path — text input + Browse button that uploads and fills the path
            const row = document.createElement('div');
            row.className = 'file-input-row';

            input = document.createElement('input');
            input.type = 'text';
            input.id = `cfg-${key}`;
            input.name = key;
            input.placeholder = '/path/to/file  or  Browse →';
            if (isRequired) input.required = true;

            const fileInput = document.createElement('input');
            fileInput.type = 'file';
            fileInput.id = `cfg-${key}-picker`;
            fileInput.style.display = 'none';
            if (key === 'file_path' || key.endsWith('_path')) {
                fileInput.accept = '.csv,.tsv,.txt,text/plain,text/csv';
            }

            const browseBtn = document.createElement('button');
            browseBtn.type = 'button';
            browseBtn.className = 'btn btn-secondary';
            browseBtn.textContent = 'Browse';
            browseBtn.onclick = () => fileInput.click();

            const statusSpan = document.createElement('span');
            statusSpan.className = 'file-upload-status';

            fileInput.addEventListener('change', async () => {
                const file = fileInput.files[0];
                if (!file) return;
                statusSpan.textContent = 'Uploading…';
                browseBtn.disabled = true;
                try {
                    const form = new FormData();
                    form.append('file', file);
                    const res = await fetch(`${API_BASE}/upload`, { method: 'POST', body: form });
                    if (!res.ok) throw new Error(`HTTP ${res.status}`);
                    const { path, size } = await res.json();
                    input.value = path;
                    statusSpan.textContent = `${file.name} · ${(size / 1024).toFixed(1)} KB`;
                    statusSpan.className = 'file-upload-status file-upload-ok';
                } catch (e) {
                    statusSpan.textContent = 'Upload failed';
                    statusSpan.className = 'file-upload-status file-upload-err';
                } finally {
                    browseBtn.disabled = false;
                }
            });

            row.append(input, browseBtn, fileInput);
            group.append(labelEl, row, statusSpan);
        } else {
            // string — check maxLength === 1 → single char input
            input = document.createElement('input');
            input.type = 'text';
            input.id = `cfg-${key}`;
            input.name = key;
            if (prop.default !== undefined) input.value = prop.default;
            if (isRequired) input.required = true;
            if (prop.maxLength === 1) input.maxLength = 1;
            input.placeholder = desc || (isRequired ? 'Required' : 'Optional');
            if (desc) addHint(group, desc);
            group.append(labelEl, input);
        }

        container.appendChild(group);
    });
}

function addHint(group, text) {
    const hint = document.createElement('p');
    hint.className = 'field-hint';
    hint.textContent = text;
    group.appendChild(hint);
}

function renderConnectorTypeGrid() {
    const grid = el('connector-type-grid');
    if (!grid) return;
    if (!availableConnectors.length) {
        grid.innerHTML = '<p style="color:#64748b;font-size:.875rem">No connectors available.</p>';
        return;
    }
    grid.innerHTML = availableConnectors.map(c => `
        <button class="btn btn-secondary node-btn"
                onclick="handleConnectorTypeSelect('${c.type}', '${c.display_name}')">
            <span class="node-btn-name">${c.display_name}</span>
            <span class="node-btn-desc">${c.description}</span>
        </button>`).join('');
}

function collectSchemaConfig(schema) {
    if (!schema?.properties) return {};
    const config = {};
    Object.entries(schema.properties).forEach(([key, prop]) => {
        const input = el(`cfg-${key}`);
        if (!input) return;
        const type = prop.type ?? 'string';
        if (type === 'boolean') {
            config[key] = input.checked;
        } else if (type === 'number' || type === 'integer') {
            const v = input.value.trim();
            if (v !== '') config[key] = Number(v);
        } else {
            const v = input.value.trim();
            if (v !== '') config[key] = v;
        }
    });
    return config;
}

async function submitAddNode(event) {
    event?.preventDefault();
    if (!selectedPipeline || !selectedCategory) return;

    const name = el('node-name').value.trim();
    if (!name) { showNotification('Name is required', 'error'); return; }

    const config = collectSchemaConfig(selectedConnMeta?.config_schema ?? null);

    const nodeTypeMap = {
        connector:      { kind: 'connector', max_outputs: null },
        transformation: { kind: 'transformation', min_inputs: 1, max_inputs: null, min_outputs: 1, max_outputs: null },
        destination:    { kind: 'destination' },
    };

    try {
        const res = await fetch(`${API_BASE}/pipelines/${selectedPipeline.id}/nodes`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
                id: 0,
                pipeline_id: selectedPipeline.id,
                node_type: nodeTypeMap[selectedCategory],
                name,
                config,
                constraints: null,
            }),
        });
        if (!res.ok) throw new Error(`HTTP ${res.status}`);
        const created = await res.json();

        selectedPipeline.nodes.push(created);
        const idx = pipelines.findIndex(p => p.id === selectedPipeline.id);
        if (idx >= 0) pipelines[idx] = selectedPipeline;

        ensurePositions(selectedPipeline.nodes);
        el('nodes-layer')?.appendChild(buildNodeElement(created));

        elements.canvasSubtitle.textContent =
            `${selectedPipeline.nodes.length} nodes · ${selectedPipeline.edges?.length ?? 0} connections`;
        closeAddNodeModal();
        showNotification('Node added', 'success');
    } catch {
        showNotification('Failed to add node', 'error');
    }
}

// ─── Validate + Run ────────────────────────────────────────────────────────
async function validatePipeline() {
    if (!selectedPipeline) return;
    try {
        const res = await fetch(`${API_BASE}/pipelines/${selectedPipeline.id}/validate`);
        const result = await res.json();
        if (result.valid) {
            showNotification('Pipeline is valid', 'success');
        } else {
            result.errors.forEach(e => showNotification(e, 'error'));
        }
    } catch {
        showNotification('Validation failed', 'error');
    }
}

async function triggerPipeline() {
    if (!selectedPipeline) return;
    try {
        const res = await fetch(`${API_BASE}/pipelines/${selectedPipeline.id}/trigger`, {
            method: 'POST',
        });
        const result = await res.json();
        if (result.valid) {
            selectedPipeline.status = 'running';
            const idx = pipelines.findIndex(p => p.id === selectedPipeline.id);
            if (idx >= 0) pipelines[idx].status = 'running';
            document.querySelector(`.pipeline-item[data-id="${selectedPipeline.id}"] .pipeline-status`)
                ?.setAttribute('class', 'pipeline-status running');
            updateStatusSummary();
            showNotification('Pipeline started', 'success');
        } else {
            result.errors.forEach(e => showNotification(e, 'error'));
        }
    } catch {
        showNotification('Failed to trigger pipeline', 'error');
    }
}

// ─── Create Pipeline modal ─────────────────────────────────────────────────
function openCreateModal() {
    elements.modal.classList.add('active');
    elements.pipelineNameInput.focus();
}

function closeCreateModal() {
    elements.modal.classList.remove('active');
    elements.pipelineForm.reset();
}

async function handleCreatePipeline(event) {
    event.preventDefault();
    const name = elements.pipelineNameInput.value.trim();
    if (!name) return;
    try {
        const pipeline = await createPipeline(name);
        await loadPipelines();
        selectPipeline(pipeline.id);
        closeCreateModal();
        showNotification('Pipeline created', 'success');
    } catch {
        showNotification('Failed to create pipeline', 'error');
    }
}

// ─── Search ────────────────────────────────────────────────────────────────
function handleSearch(event) {
    const q = event.target.value.toLowerCase().trim();
    filteredPipelines = q
        ? pipelines.filter(p => p.name.toLowerCase().includes(q))
        : [...pipelines];
    renderPipelineList();
    updateStatusSummary();
}

// ─── Notifications ─────────────────────────────────────────────────────────
function showNotification(message, type = 'info') {
    const n = document.createElement('div');
    n.textContent = message;
    n.style.cssText = `
        position:fixed; top:20px; right:20px; padding:12px 18px;
        border-radius:8px; color:#fff; font-weight:600; font-size:.875rem;
        z-index:3000; animation:slideIn .25s ease;
        background:${{ error: '#ef4444', success: '#10b981' }[type] ?? '#3b82f6'};
        box-shadow:0 8px 24px rgba(0,0,0,.35);`;
    document.body.appendChild(n);
    setTimeout(() => n.remove(), 3000);
}

// ─── Bootstrap ─────────────────────────────────────────────────────────────
document.addEventListener('DOMContentLoaded', () => {
    elements.searchInput?.addEventListener('input', handleSearch);

    el('refresh-btn')?.addEventListener('click', loadPipelines);
    el('new-pipeline-btn')?.addEventListener('click', openCreateModal);
    el('create-first-pipeline')?.addEventListener('click', openCreateModal);
    el('close-modal')?.addEventListener('click', closeCreateModal);
    el('cancel-create')?.addEventListener('click', closeCreateModal);
    el('create-pipeline')?.addEventListener('click', handleCreatePipeline);
    elements.pipelineForm?.addEventListener('submit', handleCreatePipeline);
    elements.modal?.addEventListener('click', e => { if (e.target === elements.modal) closeCreateModal(); });

    el('add-node-btn')?.addEventListener('click', openAddNodeModal);
    el('validate-btn')?.addEventListener('click', validatePipeline);
    el('run-btn')?.addEventListener('click', triggerPipeline);
    el('close-node-modal')?.addEventListener('click', closeAddNodeModal);
    el('cancel-add-node')?.addEventListener('click', closeAddNodeModal);
    el('confirm-add-node')?.addEventListener('click', submitAddNode);
    el('add-node-modal')?.addEventListener('click', e => {
        if (e.target === el('add-node-modal')) closeAddNodeModal();
    });

    // Zoom buttons
    const applyZoom = factor => {
        const svg  = elements.pipelineCanvas;
        const rect = svg.getBoundingClientRect();
        const cx = rect.width / 2, cy = rect.height / 2;
        viewTranslate.x = cx - (cx - viewTranslate.x) * factor;
        viewTranslate.y = cy - (cy - viewTranslate.y) * factor;
        viewScale = Math.min(Math.max(viewScale * factor, 0.15), 5);
        updateViewport();
    };
    el('zoom-in')?.addEventListener('click',  () => applyZoom(1.2));
    el('zoom-out')?.addEventListener('click', () => applyZoom(1 / 1.2));
    el('fit-view')?.addEventListener('click', () => {
        viewTranslate = { x: 60, y: 60 };
        viewScale = 1;
        updateViewport();
    });

    document.addEventListener('keydown', e => {
        if (e.key === 'Escape') {
            if (elements.modal?.classList.contains('active'))           closeCreateModal();
            if (el('add-node-modal')?.classList.contains('active'))     closeAddNodeModal();
        }
        if ((e.ctrlKey || e.metaKey) && e.key === 'n') { e.preventDefault(); openCreateModal(); }
        if ((e.ctrlKey || e.metaKey) && e.key === 'r') { e.preventDefault(); loadPipelines(); }
    });

    initCanvasEvents();
    fetchConnectors();
    loadPipelines();
});
