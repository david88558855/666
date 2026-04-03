package service

import (
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"regexp"
	"strconv"
	"strings"
	"sync"
	"time"

	"github.com/katelyatv/katelyatv-go/internal/model"
)

var (
	// 猫七七影视 (兼容大部分标准接口)
	defaultSources = map[string]string{
		"默认": "https://maotv.app/api.php/provide/vod",
	}
)

// SearchService 搜索服务
type SearchService struct {
	sourceService *SourceService
	cacheService  *CacheService
	httpClient    *http.Client
}

// NewSearchService 创建搜索服务
func NewSearchService(sourceService *SourceService, cacheService *CacheService) *SearchService {
	return &SearchService{
		sourceService: sourceService,
		cacheService:  cacheService,
		httpClient: &http.Client{
			Timeout: 30 * time.Second,
		},
	}
}

// Search 搜索视频
func (s *SearchService) Search(query string, filterAdult bool) ([]model.SearchItem, error) {
	// 构建缓存键
	cacheKey := fmt.Sprintf("search:%s:adult:%v", query, filterAdult)

	// 尝试从缓存获取
	if cached, err := s.cacheService.Get(cacheKey); err == nil {
		var results []model.SearchItem
		if json.Unmarshal([]byte(cached), &results) == nil {
			return results, nil
		}
	}

	// 获取所有启用的视频源
	sources, err := s.sourceService.GetEnabled()
	if err != nil || len(sources) == 0 {
		// 使用默认源
		sources = s.getDefaultSources()
	}

	// 并发搜索所有源
	var wg sync.WaitGroup
	var mu sync.Mutex
	results := make([]model.SearchItem, 0)

	for _, source := range sources {
		// 成人过滤
		if filterAdult && source.IsAdult {
			continue
		}

		wg.Add(1)
		go func(src *model.VideoSource) {
			defer wg.Done()

			items, err := s.searchSource(src, query)
			if err != nil {
				return
			}

			mu.Lock()
			results = append(results, items...)
			mu.Unlock()
		}(source)
	}

	wg.Wait()

	// 缓存结果
	if data, err := json.Marshal(results); err == nil {
		s.cacheService.Set(cacheKey, string(data), 1800) // 缓存30分钟
	}

	return results, nil
}

// searchSource 搜索单个源
func (s *SearchService) searchSource(source *model.VideoSource, query string) ([]model.SearchItem, error) {
	// 参考 MoonTV：使用 ac=detail&wd=关键词 进行搜索
	apiURL := source.API
	if !strings.Contains(apiURL, "?") {
		apiURL += "/?ac=detail&wd=" + url.QueryEscape(query)
	} else {
		apiURL += "&ac=detail&wd=" + url.QueryEscape(query)
	}

	req, err := http.NewRequest("GET", apiURL, nil)
	if err != nil {
		return nil, err
	}

	req.Header.Set("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")

	resp, err := s.httpClient.Do(req)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()

	body, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, err
	}

	return s.parseVideoList(source, string(body))
}

// parseVideoList 解析视频列表（vod_id 可能是 int 或 string，使用 json.Number 兼容）
func (s *SearchService) parseVideoList(source *model.VideoSource, body string) ([]model.SearchItem, error) {
	var items []model.SearchItem

	// 尝试JSON格式解析 - 使用 json.Number 兼容 int/string 类型的 vod_id
	if strings.Contains(body, `"list"`) {
		// 先用通用结构解析，vod_id 用 interface{} 接收
		var result struct {
			List []map[string]interface{} `json:"list"`
		}

		if err := json.Unmarshal([]byte(body), &result); err == nil {
			for _, v := range result.List {
				// 提取 vod_id，可能是 int/float64/string
				var vodID string
				switch id := v["vod_id"].(type) {
				case float64:
					vodID = strconv.FormatInt(int64(id), 10)
				case string:
					vodID = id
				case json.Number:
					vodID = id.String()
				default:
					continue
				}

				vodName, _ := v["vod_name"].(string)
				vodPic, _ := v["vod_pic"].(string)
				vodType, _ := v["vod_type"].(string)
				vodYear, _ := v["vod_year"].(string)
				vodNote, _ := v["vod_note"].(string)

				item := model.SearchItem{
					ID:       vodID,
					Title:    vodName,
					Cover:    vodPic,
					Type:     vodType,
					Site:     source.API,
					SiteName: source.Name,
					Year:     vodYear,
					Note:     vodNote,
				}
				// 判断类型
				typeNum, _ := strconv.Atoi(vodType)
				switch typeNum {
				case 1:
					item.Type = "movie"
				case 2:
					item.Type = "tv"
				default:
					if strings.Contains(vodType, "电影") {
						item.Type = "movie"
					} else {
						item.Type = "tv"
					}
				}
				items = append(items, item)
			}
			return items, nil
		}
	}

	// XML格式解析
	items = s.parseXML(source, body)

	return items, nil
}

// parseXML 解析XML格式
func (s *SearchService) parseXML(source *model.VideoSource, body string) []model.SearchItem {
	var items []model.SearchItem

	// 简单XML解析
	videoPattern := regexp.MustCompile(`<video>\s*<id>([^<]+)</id>\s*<name>([^<]+)</name>\s*<pic>([^<]+)</pic>`)
	matches := videoPattern.FindAllStringSubmatch(body, -1)

	for _, match := range matches {
		if len(match) >= 4 {
			item := model.SearchItem{
				ID:       match[1],
				Title:    match[2],
				Cover:    match[3],
				Site:     source.API,
				SiteName: source.Name,
				Type:     "movie",
			}
			items = append(items, item)
		}
	}

	return items
}

// GetDetail 获取视频详情
func (s *SearchService) GetDetail(siteAPI, videoID string) (*model.DetailData, error) {
	cacheKey := fmt.Sprintf("detail:%s:%s", siteAPI, videoID)

	// 尝试从缓存获取
	if cached, err := s.cacheService.Get(cacheKey); err == nil {
		var detail model.DetailData
		if json.Unmarshal([]byte(cached), &detail) == nil {
			return &detail, nil
		}
	}

	// 构建详情URL
	apiURL := siteAPI
	if !strings.Contains(apiURL, "?") {
		apiURL += "/?ac=detail"
	} else {
		apiURL += "&ac=detail"
	}
	apiURL += "&ids=" + videoID

	resp, err := s.httpClient.Get(apiURL)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()

	body, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, err
	}

	detail, err := s.parseDetail(siteAPI, string(body))
	if err != nil {
		return nil, err
	}

	// 缓存结果
	if data, err := json.Marshal(detail); err == nil {
		s.cacheService.Set(cacheKey, string(data), 3600) // 缓存1小时
	}

	return detail, nil
}

// parseDetail 解析详情
func (s *SearchService) parseDetail(siteAPI, body string) (*model.DetailData, error) {
	// 尝试JSON格式解析 - vod_id 用 interface{} 兼容 int/string
	if strings.Contains(body, `"list"`) {
		var result struct {
			List []map[string]interface{} `json:"list"`
		}

		if err := json.Unmarshal([]byte(body), &result); err == nil && len(result.List) > 0 {
			v := result.List[0]

			// 提取 vod_id
			var vodID string
			switch id := v["vod_id"].(type) {
			case float64:
				vodID = strconv.FormatInt(int64(id), 10)
			case string:
				vodID = id
			case json.Number:
				vodID = id.String()
			}

			vodName, _ := v["vod_name"].(string)
			vodPic, _ := v["vod_pic"].(string)
			vodType, _ := v["vod_type"].(string)
			vodYear, _ := v["vod_year"].(string)
			vodArea, _ := v["vod_area"].(string)
			vodLang, _ := v["vod_lang"].(string)
			vodDirector, _ := v["vod_director"].(string)
			vodActor, _ := v["vod_actor"].(string)
			vodContent, _ := v["vod_content"].(string)
			vodPlayFrom, _ := v["vod_play_from"].(string)
			vodPlayURL, _ := v["vod_play_url"].(string)

			// 解析播放列表
			episodes := s.parsePlayList(vodPlayFrom, vodPlayURL)

			return &model.DetailData{
				ID:       vodID,
				Title:    vodName,
				Cover:    vodPic,
				Type:     vodType,
				Year:     vodYear,
				Area:     vodArea,
				Lang:     vodLang,
				Director: vodDirector,
				Actor:    vodActor,
				Desc:     vodContent,
				Site:     siteAPI,
				Episodes: episodes,
			}, nil
		}
	}

	// XML解析
	return s.parseDetailXML(siteAPI, body)
}

// parseDetailXML 解析XML详情
func (s *SearchService) parseDetailXML(siteAPI, body string) (*model.DetailData, error) {
	id := extractXML(body, "id")
	name := extractXML(body, "name")
	pic := extractXML(body, "pic")

	return &model.DetailData{
		ID:       id,
		Title:    name,
		Cover:    pic,
		Site:     siteAPI,
		Episodes: []model.Episode{},
	}, nil
}

// parsePlayList 解析播放列表
// 格式1: vod_play_from = "bfzy$m3u8$$$lzm3u8$m3u8" (源名$解析器)
//        vod_play_url = "第01集$url#第02集$url$$$第01集$url#第02集$url"
// 格式2: vod_play_from = "jsyun$$$jsm3u8" (多个源名)
//        vod_play_url = "第01集$url#第02集$url" (单源数据，无$$$分隔)
func (s *SearchService) parsePlayList(from, playURL string) []model.Episode {
	var episodes []model.Episode

	if playURL == "" {
		return episodes
	}

	// 检查 playURL 是否包含 $$$ (多源分隔符)
	if strings.Contains(playURL, "$$$") {
		// 多源格式：源和URL都用 $$$ 分隔
		sources := strings.Split(from, "$$$")
		urls := strings.Split(playURL, "$$$")

		for i, source := range sources {
			if i >= len(urls) {
				break
			}

			// 解析源名 - 格式: "源名$解析器" 或 "源名"
			sourceName := source
			if idx := strings.Index(source, "$"); idx > 0 {
				sourceName = source[:idx]
			}

			// 分割单集 - 用 # 分隔不同集
			episodes = append(episodes, s.parseEpisodes(sourceName, urls[i])...)
		}
	} else {
		// 单源格式：只有 playURL，from 可能有多个源名但数据是合并的
		// 或者 from 为空，直接解析 playURL
		sources := strings.Split(from, "$$$")
		sourceName := ""
		if len(sources) > 0 && sources[0] != "" {
			sourceName = sources[0]
			if idx := strings.Index(sourceName, "$"); idx > 0 {
				sourceName = sourceName[:idx]
			}
		}
		if sourceName == "" {
			sourceName = "默认"
		}

		episodes = s.parseEpisodes(sourceName, playURL)
	}

	return episodes
}

// parseEpisodes 解析单个源的剧集列表
func (s *SearchService) parseEpisodes(sourceName, data string) []model.Episode {
	var episodes []model.Episode

	if data == "" {
		return episodes
	}

	// 分割单集 - 用 # 分隔不同集
	parts := strings.Split(data, "#")
	for _, part := range parts {
		if part == "" {
			continue
		}
		// 格式: "集名$url" 或 "url" (有些源没有集名)
		eps := strings.SplitN(part, "$", 2)
		if len(eps) >= 2 && eps[1] != "" {
			epName := strings.TrimSpace(eps[0])
			if epName == "" {
				epName = fmt.Sprintf("第%d集", len(episodes)+1)
			}
			// 清理 URL 中可能的制表符
			playURL := strings.TrimSpace(eps[1])
			episodes = append(episodes, model.Episode{
				EpisodeID: fmt.Sprintf("%d", len(episodes)),
				Name:      fmt.Sprintf("[%s] %s", sourceName, epName),
				PlayURL:   playURL,
			})
		} else if len(eps) == 1 && eps[0] != "" && (strings.HasPrefix(eps[0], "http") || strings.Contains(eps[0], ".m3u8")) {
			// 没有$分隔符，整个就是URL
			episodes = append(episodes, model.Episode{
				EpisodeID: fmt.Sprintf("%d", len(episodes)),
				Name:      fmt.Sprintf("[%s] 第%d集", sourceName, len(episodes)+1),
				PlayURL:   strings.TrimSpace(eps[0]),
			})
		}
	}

	return episodes
}

// GetPlayUrl 获取播放地址
func (s *SearchService) GetPlayUrl(siteAPI, episodeID string) (*model.PlayData, error) {
	// episodeID 格式: "sourceIndex-episodeIndex"
	// 需要先获取详情再返回对应集数的URL
	return &model.PlayData{
		URL:     "",
		Headers: nil,
	}, nil
}

// ParsePlayURL 解析实际播放URL
func (s *SearchService) ParsePlayURL(rawURL string) (*model.PlayData, error) {
	// 对于直链，直接返回
	if strings.HasPrefix(rawURL, "http") {
		return &model.PlayData{
			URL: rawURL,
		}, nil
	}

	// M3U8 处理
	if strings.Contains(rawURL, ".m3u8") {
		return &model.PlayData{
			URL: rawURL,
		}, nil
	}

	// 需要代理的URL
	return &model.PlayData{
		URL: rawURL,
	}, nil
}

// GetCategories 获取分类
func (s *SearchService) GetCategories() []map[string]string {
	return []map[string]string{
		{"id": "1", "name": "电影"},
		{"id": "2", "name": "电视剧"},
		{"id": "3", "name": "综艺"},
		{"id": "4", "name": "动漫"},
		{"id": "5", "name": "纪录片"},
	}
}

// GetHomeData 获取首页数据，并发从多个源获取并合并
func (s *SearchService) GetHomeData() (*model.HomeData, error) {
	// 获取启用的视频源
	sources, err := s.sourceService.GetEnabled()
	if err != nil || len(sources) == 0 {
		// 使用默认源
		sources = s.getDefaultSources()
	}

	var mu sync.Mutex
	var allItems []model.SearchItem
	seen := make(map[string]bool) // 去重
	var wg sync.WaitGroup

	// 并发从所有源获取数据
	for _, source := range sources {
		wg.Add(1)
		go func(src *model.VideoSource) {
			defer wg.Done()

			items, err := s.fetchHomeList(src)
			if err != nil || len(items) == 0 {
				return
			}

			mu.Lock()
			// 合并并去重
			for _, item := range items {
				key := item.Title + item.Year
				if !seen[key] {
					seen[key] = true
					allItems = append(allItems, item)
				}
			}
			mu.Unlock()
		}(source)
	}

	wg.Wait()

	// 取前24个作为热门
	hotItems := allItems
	if len(hotItems) > 24 {
		hotItems = hotItems[:24]
	}

	// 取后面的作为最新
	var newItems []model.SearchItem
	if len(allItems) > 24 {
		newItems = allItems[24:]
		if len(newItems) > 24 {
			newItems = newItems[:24]
		}
	}

	return &model.HomeData{
		Banner: []model.BannerItem{},
		Hot:    hotItems,
		New:    newItems,
	}, nil
}

// fetchHomeList 从源获取首页列表
func (s *SearchService) fetchHomeList(source *model.VideoSource) ([]model.SearchItem, error) {
	// 构建API URL - 获取最新视频列表
	apiURL := source.API
	if !strings.Contains(apiURL, "?") {
		apiURL += "/?ac=videolist&pg=1"
	} else {
		apiURL += "&ac=videolist&pg=1"
	}

	req, err := http.NewRequest("GET", apiURL, nil)
	if err != nil {
		return nil, err
	}

	req.Header.Set("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")

	resp, err := s.httpClient.Do(req)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()

	body, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, err
	}

	return s.parseHomeList(source, string(body))
}

// parseHomeList 解析首页列表（vod_id 可能是 int 或 string，使用 interface{} 兼容）
func (s *SearchService) parseHomeList(source *model.VideoSource, body string) ([]model.SearchItem, error) {
	var items []model.SearchItem

	// 尝试JSON格式解析 - vod_id 用 interface{} 兼容 int/string
	if strings.Contains(body, `"list"`) {
		var result struct {
			List []map[string]interface{} `json:"list"`
		}

		if err := json.Unmarshal([]byte(body), &result); err == nil {
			for _, v := range result.List {
				// 提取 vod_id
				var vodID string
				switch id := v["vod_id"].(type) {
				case float64:
					vodID = strconv.FormatInt(int64(id), 10)
				case string:
					vodID = id
				case json.Number:
					vodID = id.String()
				default:
					continue
				}

				vodName, _ := v["vod_name"].(string)
				vodPic, _ := v["vod_pic"].(string)
				vodType, _ := v["vod_type"].(string)
				vodYear, _ := v["vod_year"].(string)
				vodNote, _ := v["vod_note"].(string)

				item := model.SearchItem{
					ID:       vodID,
					Title:    vodName,
					Cover:    vodPic,
					Type:     vodType,
					Site:     source.API,
					SiteName: source.Name,
					Year:     vodYear,
					Note:     vodNote,
				}
				// 判断类型
				typeNum, _ := strconv.Atoi(vodType)
				switch typeNum {
				case 1:
					item.Type = "movie"
				case 2:
					item.Type = "tv"
				default:
					if strings.Contains(vodType, "电影") {
						item.Type = "movie"
					} else {
						item.Type = "tv"
					}
				}
				items = append(items, item)
			}
			return items, nil
		}
	}

	// XML格式解析
	items = s.parseHomeXML(source, body)

	return items, nil
}

// parseHomeXML 解析XML格式首页
func (s *SearchService) parseHomeXML(source *model.VideoSource, body string) []model.SearchItem {
	var items []model.SearchItem

	// 简单XML解析
	videoPattern := regexp.MustCompile(`<video>\s*<id>([^<]+)</id>\s*<name>([^<]+)</name>\s*<pic>([^<]+)</pic>`)
	matches := videoPattern.FindAllStringSubmatch(body, -1)

	for _, match := range matches {
		if len(match) >= 4 {
			item := model.SearchItem{
				ID:       match[1],
				Title:    match[2],
				Cover:    match[3],
				Site:     source.API,
				SiteName: source.Name,
				Type:     "movie",
			}
			items = append(items, item)
		}
	}

	return items
}

// getDefaultSources 获取默认源
func (s *SearchService) getDefaultSources() []*model.VideoSource {
	sources := make([]*model.VideoSource, 0, len(defaultSources))
	for name, api := range defaultSources {
		sources = append(sources, &model.VideoSource{
			Name:  name,
			API:   api,
		})
	}
	return sources
}

// extractXML 提取XML标签内容
func extractXML(body, tag string) string {
	pattern := regexp.MustCompile("<" + tag + ">([^<]*)</" + tag + ">")
	matches := pattern.FindStringSubmatch(body)
	if len(matches) >= 2 {
		return matches[1]
	}
	return ""
}

// ProxyURL 代理URL（用于播放外部视频）
func (s *SearchService) ProxyURL(targetURL string) (string, error) {
	_, err := url.Parse(targetURL)
	if err != nil {
		return "", err
	}
	return fmt.Sprintf("/api/proxy?url=%s", url.QueryEscape(targetURL)), nil
}

// FetchWithProxy 带代理的请求
func (s *SearchService) FetchWithProxy(proxyURL string) ([]byte, error) {
	resp, err := s.httpClient.Get(proxyURL)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()

	return io.ReadAll(resp.Body)
}
