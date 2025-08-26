// API Configuration - using relative URLs since served from same server
const API_BASE = '/api';

// Global state
let pipelines = [];
let filteredPipelines = [];
let selectedPipeline = null;
let currentZoom = 1;

// DOM Elements
const elements = {
    pipelineList: document.getElementById('pipeline-list'),
    canvasTitle: document.getElementById('canvas-title'),
    canvasSubtitle: document.getElementById('canvas-subtitle'),
    pipelineCanvas: document.getElementById('pipeline-canvas'),
    emptyState: document.getElementById('empty-state'),
    modal: document.getElementById('new-pipeline-modal'),
    pipelineForm: document.getElementById('new-pipeline-form'),
    pipelineNameInput: document.getElementById('pipeline-name'),
    runningCount: document.getElementById('running-count'),
    idleCount: document.getElementById('idle-count'),
    errorCount: document.getElementById('error-count'),
    searchInput: document.getElementById('pipeline-search')
};

// API Functions
async function fetchPipelines() {
    try {
        console.log('Fetching pipelines from:', `${API_BASE}/pipelines`);
        const response = await fetch(`${API_BASE}/pipelines`);
        if (!response.ok) throw new Error(`HTTP ${response.status}`);
        const data = await response.json();
        console.log('Fetched pipelines:', data);
        return data;
    } catch (error) {
        console.error('Failed to fetch pipelines:', error);
        showNotification('Failed to load pipelines', 'error');
        return [];
    }
}

async function createPipeline(name) {
    try {
        console.log('Creating pipeline:', name);
        const response = await fetch(`${API_BASE}/pipelines`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify({
                id: 0,
                name: name,
                nodes: [],
                edges: []
            })
        });
        
        if (!response.ok) throw new Error(`HTTP ${response.status}`);
        const data = await response.json();
        console.log('Created pipeline:', data);
        return data;
    } catch (error) {
        console.error('Failed to create pipeline:', error);
        showNotification('Failed to create pipeline', 'error');
        return null;
    }
}

async function fetchPipeline(id) {
    try {
        const response = await fetch(`${API_BASE}/pipelines/${id}`);
        if (!response.ok) throw new Error(`HTTP ${response.status}`);
        return await response.json();
    } catch (error) {
        console.error('Failed to fetch pipeline:', error);
        showNotification('Failed to load pipeline details', 'error');
        return null;
    }
}

// UI Functions
function renderPipelineList() {
    const pipelineList = document.getElementById('pipeline-list');
    if (!pipelineList) return;

    if (filteredPipelines.length === 0) {
        const isSearching = elements.searchInput && elements.searchInput.value.trim() !== '';
        
        pipelineList.innerHTML = `
            <div class="empty-list">
                <p>${isSearching ? 'No pipelines match your search' : 'No pipelines found'}</p>
                ${!isSearching ? '<button class="btn btn-primary" onclick="openCreateModal()">Create First Pipeline</button>' : ''}
            </div>
        `;
        return;
    }

    pipelineList.innerHTML = filteredPipelines.map(pipeline => {
        const status = getRandomStatus(); // TODO: Use real status from API
        const nodeCount = pipeline.nodes ? pipeline.nodes.length : 0;
        
        return `
            <div class="pipeline-item ${selectedPipeline && selectedPipeline.id === pipeline.id ? 'active' : ''}" 
                 onclick="selectPipeline(${pipeline.id})">
                <div class="pipeline-name" title="${pipeline.name}">${pipeline.name}</div>
                <div class="pipeline-meta">
                    <div class="pipeline-status ${status}"></div>
                    <span>${nodeCount}</span>
                </div>
            </div>
        `;
    }).join('');
}

function updateStatusSummary() {
    const runningCount = document.getElementById('running-count');
    const idleCount = document.getElementById('idle-count');
    const errorCount = document.getElementById('error-count');

    // Count statuses from filtered pipelines (for search results)
    let running = 0, idle = 0, error = 0;
    
    filteredPipelines.forEach(() => {
        const status = getRandomStatus(); // TODO: Use real status
        switch (status) {
            case 'running': running++; break;
            case 'idle': idle++; break;
            case 'error': error++; break;
        }
    });

    if (runningCount) runningCount.textContent = running;
    if (idleCount) idleCount.textContent = idle;
    if (errorCount) errorCount.textContent = error;
}

function selectPipeline(pipeline) {
    console.log('Selecting pipeline:', pipeline);
    selectedPipeline = pipeline;
    renderPipelineList(); // Re-render to update active state
    renderPipelineCanvas(pipeline);
    
    elements.canvasTitle.textContent = pipeline.name;
    elements.canvasSubtitle.textContent = `${pipeline.nodes?.length || 0} nodes, ${pipeline.edges?.length || 0} connections`;
    elements.emptyState.style.display = 'none';
}

function renderPipelineCanvas(pipeline) {
    const canvas = elements.pipelineCanvas;
    canvas.innerHTML = ''; // Clear existing content
    
    if (!pipeline.nodes || pipeline.nodes.length === 0) {
        // Show empty pipeline state
        const emptyGroup = document.createElementNS('http://www.w3.org/2000/svg', 'g');
        emptyGroup.innerHTML = `
            <text x="50%" y="50%" text-anchor="middle" fill="#9ca3af" font-size="16">
                This pipeline is empty. Add nodes to get started.
            </text>
        `;
        canvas.appendChild(emptyGroup);
        return;
    }
    
    // Simple node layout (we'll enhance this later)
    const nodeSpacing = 200;
    const nodeRadius = 30;
    const startX = 100;
    const startY = 100;
    
    // Render nodes
    pipeline.nodes.forEach((node, index) => {
        const x = startX + (index * nodeSpacing);
        const y = startY;
        
        // Node circle
        const nodeGroup = document.createElementNS('http://www.w3.org/2000/svg', 'g');
        nodeGroup.setAttribute('class', 'node');
        nodeGroup.setAttribute('transform', `translate(${x}, ${y})`);
        
        const circle = document.createElementNS('http://www.w3.org/2000/svg', 'circle');
        circle.setAttribute('r', nodeRadius);
        circle.setAttribute('fill', getNodeColor(node.node_type));
        circle.setAttribute('stroke', '#374151');
        circle.setAttribute('stroke-width', '2');
        
        const text = document.createElementNS('http://www.w3.org/2000/svg', 'text');
        text.setAttribute('text-anchor', 'middle');
        text.setAttribute('dy', '0.35em');
        text.setAttribute('fill', 'white');
        text.setAttribute('font-size', '12');
        text.setAttribute('font-weight', 'bold');
        text.textContent = node.name.substring(0, 8);
        
        nodeGroup.appendChild(circle);
        nodeGroup.appendChild(text);
        canvas.appendChild(nodeGroup);
    });
    
    // Render edges (simple lines for now)
    if (pipeline.edges) {
        pipeline.edges.forEach(edge => {
            const fromIndex = pipeline.nodes.findIndex(n => n.id === edge.from);
            const toIndex = pipeline.nodes.findIndex(n => n.id === edge.to);
            
            if (fromIndex >= 0 && toIndex >= 0) {
                const fromX = startX + (fromIndex * nodeSpacing);
                const toX = startX + (toIndex * nodeSpacing);
                const y = startY;
                
                const line = document.createElementNS('http://www.w3.org/2000/svg', 'line');
                line.setAttribute('x1', fromX + nodeRadius);
                line.setAttribute('y1', y);
                line.setAttribute('x2', toX - nodeRadius);
                line.setAttribute('y2', y);
                line.setAttribute('stroke', '#6b7280');
                line.setAttribute('stroke-width', '2');
                line.setAttribute('marker-end', 'url(#arrowhead)');
                
                canvas.appendChild(line);
            }
        });
    }
    
    // Add arrow marker definition
    const defs = document.createElementNS('http://www.w3.org/2000/svg', 'defs');
    defs.innerHTML = `
        <marker id="arrowhead" markerWidth="10" markerHeight="7" 
                refX="9" refY="3.5" orient="auto">
            <polygon points="0 0, 10 3.5, 0 7" fill="#6b7280" />
        </marker>
    `;
    canvas.appendChild(defs);
}

function getNodeColor(nodeType) {
    const colors = {
        'Connector': '#3b82f6',
        'Transformation': '#10b981', 
        'Destination': '#f59e0b'
    };
    return colors[nodeType] || '#6b7280';
}

function openCreateModal() {
    console.log('Opening create modal');
    elements.modal.classList.add('active');
    elements.pipelineNameInput.focus();
}

function closeCreateModal() {
    console.log('Closing create modal');
    elements.modal.classList.remove('active');
    elements.pipelineForm.reset();
}

async function handleCreatePipeline(event) {
    event.preventDefault();
    console.log('Handling create pipeline');
    
    const name = elements.pipelineNameInput.value.trim();
    if (!name) return;
    
    const newPipeline = await createPipeline(name);
    if (newPipeline) {
        await loadPipelines();
        selectPipeline(newPipeline);
        closeCreateModal();
        showNotification('Pipeline created successfully', 'success');
    }
}

async function loadPipelines() {
    console.log('Loading pipelines...');
    try {
        console.log('Fetching pipelines from:', `${API_BASE}/pipelines`);
        const response = await fetch(`${API_BASE}/pipelines`);
        
        if (!response.ok) {
            throw new Error(`HTTP error! status: ${response.status}`);
        }
        
        pipelines = await response.json();
        filteredPipelines = [...pipelines];
        console.log('Fetched pipelines:', pipelines);
        
        renderPipelineList();
        updateStatusSummary();
        
        // Select first pipeline if none selected
        if (pipelines.length > 0 && !selectedPipeline) {
            selectPipeline(pipelines[0].id);
        }
        
    } catch (error) {
        console.error('Error loading pipelines:', error);
        showNotification('Failed to load pipelines', 'error');
        
        // Show empty state
        const pipelineList = document.getElementById('pipeline-list');
        if (pipelineList) {
            pipelineList.innerHTML = `
                <div class="empty-list">
                    <p>Failed to load pipelines</p>
                    <button class="btn btn-secondary" onclick="loadPipelines()">Retry</button>
                </div>
            `;
        }
    }
}

function handleSearch(event) {
    const query = event.target.value.toLowerCase().trim();
    console.log('Searching for:', query);
    
    if (query === '') {
        filteredPipelines = [...pipelines];
    } else {
        filteredPipelines = pipelines.filter(pipeline => 
            pipeline.name.toLowerCase().includes(query) ||
            (pipeline.description && pipeline.description.toLowerCase().includes(query))
        );
    }
    
    renderPipelineList();
    updateStatusSummary();
}

function showNotification(message, type = 'info') {
    console.log('Showing notification:', message, type);
    // Simple notification system
    const notification = document.createElement('div');
    notification.className = `notification notification-${type}`;
    notification.textContent = message;
    notification.style.cssText = `
        position: fixed;
        top: 20px;
        right: 20px;
        padding: 12px 16px;
        border-radius: 6px;
        color: white;
        font-weight: 500;
        z-index: 2000;
        background: ${type === 'error' ? '#ef4444' : type === 'success' ? '#10b981' : '#3b82f6'};
        box-shadow: 0 4px 12px rgba(0,0,0,0.15);
    `;
    
    document.body.appendChild(notification);
    
    setTimeout(() => {
        notification.remove();
    }, 3000);
}

// Event Listeners
document.addEventListener('DOMContentLoaded', initializeApp);

function initializeApp() {
    console.log('DOM loaded, initializing app...');
    
    // Set up search functionality
    const searchInput = document.getElementById('pipeline-search');
    if (searchInput) {
        searchInput.addEventListener('input', handleSearch);
        console.log('Search functionality initialized');
    }
    
    // Load initial data
    loadPipelines();
    
    // Button event listeners
    const refreshBtn = document.getElementById('refresh-btn');
    const newPipelineBtn = document.getElementById('new-pipeline-btn');
    const createFirstBtn = document.getElementById('create-first-pipeline');
    const closeModalBtn = document.getElementById('close-modal');
    const cancelCreateBtn = document.getElementById('cancel-create');
    const createPipelineBtn = document.getElementById('create-pipeline');
    
    if (refreshBtn) refreshBtn.onclick = loadPipelines;
    if (newPipelineBtn) newPipelineBtn.onclick = openCreateModal;
    if (createFirstBtn) createFirstBtn.onclick = openCreateModal;
    if (closeModalBtn) closeModalBtn.onclick = closeCreateModal;
    if (cancelCreateBtn) cancelCreateBtn.onclick = closeCreateModal;
    if (createPipelineBtn) createPipelineBtn.onclick = handleCreatePipeline;
    
    // Form submission
    if (elements.pipelineForm) {
        elements.pipelineForm.onsubmit = handleCreatePipeline;
    }
    
    // Modal backdrop click
    if (elements.modal) {
        elements.modal.onclick = (e) => {
            if (e.target === elements.modal) closeCreateModal();
        };
    }
    
    // Canvas controls
    const zoomInBtn = document.getElementById('zoom-in');
    const zoomOutBtn = document.getElementById('zoom-out');
    const fitViewBtn = document.getElementById('fit-view');
    
    if (zoomInBtn) {
        zoomInBtn.onclick = () => {
            currentZoom = Math.min(currentZoom * 1.2, 3);
            elements.pipelineCanvas.style.transform = `scale(${currentZoom})`;
        };
    }
    
    if (zoomOutBtn) {
        zoomOutBtn.onclick = () => {
            currentZoom = Math.max(currentZoom / 1.2, 0.3);
            elements.pipelineCanvas.style.transform = `scale(${currentZoom})`;
        };
    }
    
    if (fitViewBtn) {
        fitViewBtn.onclick = () => {
            currentZoom = 1;
            elements.pipelineCanvas.style.transform = 'scale(1)';
        };
    }
}

// Keyboard shortcuts
document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape' && elements.modal && elements.modal.classList.contains('active')) {
        closeCreateModal();
    }
    
    if (e.ctrlKey || e.metaKey) {
        if (e.key === 'n') {
            e.preventDefault();
            openCreateModal();
        }
        if (e.key === 'r') {
            e.preventDefault();
            loadPipelines();
        }
    }
});

console.log('App.js loaded successfully');

function getRandomStatus() {
    const statuses = ['running', 'idle', 'error'];
    return statuses[Math.floor(Math.random() * statuses.length)];
}
