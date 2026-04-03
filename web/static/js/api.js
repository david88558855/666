// API 模块
const API = {
    // 获取当前用户
    getUser() {
        const userStr = localStorage.getItem('user');
        if (userStr) {
            try {
                return JSON.parse(userStr);
            } catch (e) {
                return null;
            }
        }
        return null;
    },

    // 获取Token
    getToken() {
        return localStorage.getItem('token');
    },

    // 检查是否已登录
    isLoggedIn() {
        return !!this.getToken();
    },

    // 检查是否是管理员
    isAdmin() {
        const user = this.getUser();
        return user && user.is_admin;
    },

    // 登录
    async login(username, password) {
        const res = await fetch('/api/auth/login', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ username, password })
        });
        const data = await res.json();
        if (data.code === 0) {
            localStorage.setItem('user', JSON.stringify(data.data.user));
            localStorage.setItem('token', data.data.token);
        }
        return data;
    },

    // 注册
    async register(username, password) {
        const res = await fetch('/api/auth/register', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ username, password })
        });
        const data = await res.json();
        if (data.code === 0) {
            localStorage.setItem('user', JSON.stringify(data.data.user));
            localStorage.setItem('token', data.data.token);
        }
        return data;
    },

    // 登出
    logout() {
        localStorage.removeItem('user');
        localStorage.removeItem('token');
        location.href = '/';
    },

    // 获取收藏
    async getFavorites() {
        const token = this.getToken();
        if (!token) return { code: 401, message: '请先登录' };
        
        const res = await fetch('/api/user/favorites', {
            headers: { 'Authorization': 'Bearer ' + token }
        });
        return res.json();
    },

    // 添加收藏
    async addFavorite(data) {
        const token = this.getToken();
        if (!token) return { code: 401, message: '请先登录' };

        const res = await fetch('/api/user/favorites', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json', 'Authorization': 'Bearer ' + token },
            body: JSON.stringify(data)
        });
        return res.json();
    },

    // 删除收藏
    async removeFavorite(id) {
        const token = this.getToken();
        if (!token) return { code: 401, message: '请先登录' };

        const res = await fetch(`/api/user/favorites/${id}`, {
            method: 'DELETE',
            headers: { 'Authorization': 'Bearer ' + token }
        });
        return res.json();
    },

    // 获取历史
    async getHistory() {
        const token = this.getToken();
        if (!token) return { code: 401, message: '请先登录' };

        const res = await fetch('/api/user/history', {
            headers: { 'Authorization': 'Bearer ' + token }
        });
        return res.json();
    },

    // 添加历史
    async addHistory(data) {
        const token = this.getToken();
        if (!token) return { code: 401, message: '请先登录' };

        const res = await fetch('/api/user/history', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json', 'Authorization': 'Bearer ' + token },
            body: JSON.stringify(data)
        });
        return res.json();
    },

    // 搜索
    async search(query) {
        const user = this.getUser();
        const headers = {};
        if (user) headers['Authorization'] = 'Bearer ' + user.id;

        const res = await fetch(`/api/search?q=${encodeURIComponent(query)}`, { headers });
        return res.json();
    },

    // 获取详情
    async getDetail(site, id) {
        const res = await fetch(`/api/detail/${encodeURIComponent(site)}/${id}`);
        return res.json();
    }
};

window.API = API;
