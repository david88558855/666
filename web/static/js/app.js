// 主应用逻辑
const api = window.API;

document.addEventListener('DOMContentLoaded', () => {
    initApp();
});

function initApp() {
    updateAuthUI();
    initSearch();
    initNav();
    loadHomeData();
}

// 更新登录UI
function updateAuthUI() {
    const user = api.getUser();
    const loginBtn = document.getElementById('loginBtn');
    const adminBtn = document.getElementById('adminBtn');
    const logoutBtn = document.getElementById('logoutBtn');

    if (user) {
        if (loginBtn) loginBtn.style.display = 'none';
        if (logoutBtn) logoutBtn.style.display = 'block';
        if (adminBtn && user.is_admin) adminBtn.style.display = 'block';
        
        logoutBtn.addEventListener('click', () => api.logout());
    } else {
        if (loginBtn) loginBtn.style.display = 'block';
        if (adminBtn) adminBtn.style.display = 'none';
        if (logoutBtn) logoutBtn.style.display = 'none';
    }
}

// 初始化搜索
function initSearch() {
    const searchInput = document.getElementById('searchInput');
    const searchBtn = document.getElementById('searchBtn');

    if (searchBtn) {
        searchBtn.addEventListener('click', () => {
            const query = searchInput?.value?.trim();
            if (query) {
                location.href = `/search?q=${encodeURIComponent(query)}`;
            }
        });
    }

    if (searchInput) {
        searchInput.addEventListener('keypress', (e) => {
            if (e.key === 'Enter') {
                const query = searchInput.value.trim();
                if (query) {
                    location.href = `/search?q=${encodeURIComponent(query)}`;
                }
            }
        });
    }
}

// 初始化导航
function initNav() {
    const navHistory = document.getElementById('navHistory');
    const navFavorites = document.getElementById('navFavorites');

    if (navHistory) {
        navHistory.addEventListener('click', (e) => {
            e.preventDefault();
            if (!api.isLoggedIn()) {
                showToast('请先登录');
                setTimeout(() => location.href = '/login', 1000);
                return;
            }
            showSection('history');
        });
    }

    if (navFavorites) {
        navFavorites.addEventListener('click', (e) => {
            e.preventDefault();
            if (!api.isLoggedIn()) {
                showToast('请先登录');
                setTimeout(() => location.href = '/login', 1000);
                return;
            }
            showSection('favorites');
        });
    }

    // 分类导航
    document.querySelectorAll('.category-item').forEach(item => {
        item.addEventListener('click', () => {
            document.querySelectorAll('.category-item').forEach(i => i.classList.remove('active'));
            item.classList.add('active');
            const category = item.dataset.id;
            if (category !== 'all') {
                showToast(`分类: ${item.textContent}`);
            }
        });
    });
}

// 显示指定区域
function showSection(section) {
    const sections = ['searchResults', 'historySection', 'favoritesSection'];
    sections.forEach(s => {
        const el = document.getElementById(s);
        if (el) el.style.display = 'none';
    });

    if (section === 'history') {
        const el = document.getElementById('historySection');
        if (el) {
            el.style.display = 'block';
            loadHistory();
        }
    } else if (section === 'favorites') {
        const el = document.getElementById('favoritesSection');
        if (el) {
            el.style.display = 'block';
            loadFavorites();
        }
    }
}

// 加载首页数据
async function loadHomeData() {
    try {
        // 加载热门
        const hotRes = await fetch('/api/home');
        const hotData = await hotRes.json();
        if (hotData.code === 0) {
            renderVideoList('hotList', hotData.data?.hot || []);
            renderVideoList('newList', hotData.data?.new || []);
        }
    } catch (err) {
        console.error('加载首页数据失败:', err);
        // 显示空状态
        document.getElementById('hotList').innerHTML = '<p class="no-data">暂无数据，请先添加视频源</p>';
        document.getElementById('newList').innerHTML = '<p class="no-data">暂无数据</p>';
    }
}

// 加载历史记录
async function loadHistory() {
    if (!api.isLoggedIn()) return;

    const container = document.getElementById('historyList');
    container.innerHTML = '<p class="loading">加载中...</p>';

    try {
        const data = await api.getHistory();
        if (data.code === 0 && data.data && data.data.length > 0) {
            const items = data.data.map(h => ({
                id: h.video_id,
                title: h.title,
                cover: h.cover,
                type: h.type,
                site: h.site,
                site_name: h.site_name
            }));
            renderVideoList('historyList', items);
        } else {
            container.innerHTML = '<p class="no-data">暂无观看历史</p>';
        }
    } catch (err) {
        container.innerHTML = '<p class="no-data">加载失败</p>';
    }
}

// 加载收藏
async function loadFavorites() {
    if (!api.isLoggedIn()) return;

    const container = document.getElementById('favoritesList');
    container.innerHTML = '<p class="loading">加载中...</p>';

    try {
        const data = await api.getFavorites();
        if (data.code === 0 && data.data && data.data.length > 0) {
            const items = data.data.map(f => ({
                id: f.video_id,
                title: f.title,
                cover: f.cover,
                type: f.type,
                site: f.site,
                site_name: f.site_name
            }));
            renderVideoList('favoritesList', items);
        } else {
            container.innerHTML = '<p class="no-data">暂无收藏</p>';
        }
    } catch (err) {
        container.innerHTML = '<p class="no-data">加载失败</p>';
    }
}

// 渲染视频列表
function renderVideoList(containerId, items) {
    const container = document.getElementById(containerId);
    if (!container) return;

    if (!items || items.length === 0) {
        container.innerHTML = '<p class="no-data">暂无数据</p>';
        return;
    }

    container.innerHTML = items.map(item => `
        <a href="/play?site=${encodeURIComponent(item.site)}&id=${item.id}&title=${encodeURIComponent(item.title)}&cover=${encodeURIComponent(item.cover || '')}" class="video-card">
            <div class="video-cover">
                <img src="${item.cover || '/static/img/placeholder.png'}" alt="${item.title}" loading="lazy">
                <span class="video-type">${item.type === 'tv' ? '剧' : '影'}</span>
            </div>
            <div class="video-info">
                <h3 class="video-title">${item.title}</h3>
                <div class="video-meta">
                    <span>${item.site_name || ''}</span>
                </div>
            </div>
        </a>
    `).join('');
}

// Toast提示
function showToast(msg) {
    let toast = document.getElementById('toast');
    if (!toast) {
        toast = document.createElement('div');
        toast.id = 'toast';
        toast.className = 'toast';
        document.body.appendChild(toast);
    }
    toast.textContent = msg;
    toast.classList.add('show');
    setTimeout(() => toast.classList.remove('show'), 2000);
}
