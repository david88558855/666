package service

import (
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"regexp"
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

	// 获取视频源
	sources, err := s.sourceService.GetAll()
	if err != nil || len(sources) == 0 {
		// 使用默认源
		sources = s.getDefaultSources()
	}

	// 并发搜索
	var wg sync.WaitGroup
	var mu sync.Mutex
	results := make([]model.SearchItem, 0)
	errorCh := make(chan error, len(sources))

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
				errorCh <- err
				return
			}

			mu.Lock()
			results = append(results, items...)
			mu.Unlock()
		}(source)
	}

	wg.Wait()
	close(errorCh)

	// 缓存结果
	if data, err := json.Marshal(results); err == nil {
		s.cacheService.Set(cacheKey, string(data), 1800) // 缓存30分钟
	}

	return results, nil
}

// searchSource 搜索单个源
func (s *SearchService) searchSource(source *model.VideoSource, query string) ([]model.SearchItem, error) {
	apiURL := source.API
	if !strings.Contains(apiURL, "?") {
		apiURL += "/?ac=videolist"
	} else {
		apiURL += "&ac=videolist"
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

	return s.parseVideoList(source, string(body), query)
}

// parseVideoList 解析视频列表
func (s *SearchService) parseVideoList(source *model.VideoSource, body, query string) ([]model.SearchItem, error) {
	var items []model.SearchItem

	// 尝试JSON格式解析
	if strings.Contains(body, `"list"`) {
		var result struct {
			List []struct {
				VodID       string `json:"vod_id"`
				VodName     string `json:"vod_name"`
				VodPic      string `json:"vod_pic"`
				VodType     string `json:"vod_type"`
				VodYear     string `json:"vod_year"`
				VodArea     string `json:"vod_area"`
				VodNote     string `json:"vod_note"`
			} `json:"list"`
		}

		if err := json.Unmarshal([]byte(body), &result); err == nil {
			queryLower := strings.ToLower(query)
			for _, v := range result.List {
				if strings.Contains(strings.ToLower(v.VodName), queryLower) {
					item := model.SearchItem{
						ID:       v.VodID,
						Title:    v.VodName,
						Cover:    v.VodPic,
						Type:     v.VodType,
						Site:     source.API,
						SiteName: source.Name,
						Year:     v.VodYear,
						Note:     v.VodNote,
					}
					// 判断类型
					if strings.Contains(v.VodType, "剧") || strings.Contains(v.VodType, "综艺") {
						item.Type = "tv"
					} else {
						item.Type = "movie"
					}
					items = append(items, item)
				}
			}
			return items, nil
		}
	}

	// XML格式解析
	items = s.parseXML(source, body, query)

	return items, nil
}

// parseXML 解析XML格式
func (s *SearchService) parseXML(source *model.VideoSource, body, query string) []model.SearchItem {
	var items []model.SearchItem

	// 简单XML解析
	videoPattern := regexp.MustCompile(`<video>\s*<id>([^<]+)</id>\s*<name>([^<]+)</name>\s*<pic>([^<]+)</pic>`)
	matches := videoPattern.FindAllStringSubmatch(body, -1)

	queryLower := strings.ToLower(query)

	for _, match := range matches {
		if len(match) >= 4 && strings.Contains(strings.ToLower(match[2]), queryLower) {
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
	// 尝试JSON格式解析
	if strings.Contains(body, `"list"`) {
		var result struct {
			List []struct {
				VodID       string `json:"vod_id"`
				VodName     string `json:"vod_name"`
				VodPic      string `json:"vod_pic"`
				VodType     string `json:"vod_type"`
				VodYear     string `json:"vod_year"`
				VodArea     string `json:"vod_area"`
				VodLang     string `json:"vod_lang"`
				VodDirector string `json:"vod_director"`
				VodActor    string `json:"vod_actor"`
				VodContent  string `json:"vod_content"`
				VodPlayFrom string `json:"vod_play_from"`
				VodPlayURL  string `json:"vod_play_url"`
			} `json:"list"`
		}

		if err := json.Unmarshal([]byte(body), &result); err == nil && len(result.List) > 0 {
			v := result.List[0]
			
			// 解析播放列表
			episodes := s.parsePlayList(v.VodPlayFrom, v.VodPlayURL)

			return &model.DetailData{
				ID:       v.VodID,
				Title:    v.VodName,
				Cover:    v.VodPic,
				Type:     v.VodType,
				Year:     v.VodYear,
				Area:     v.VodArea,
				Lang:     v.VodLang,
				Director: v.VodDirector,
				Actor:    v.VodActor,
				Desc:     v.VodContent,
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
func (s *SearchService) parsePlayList(from, url string) []model.Episode {
	var episodes []model.Episode

	// 分割播放源
	sources := strings.Split(from, "$$$")
	urls := strings.Split(url, "$$$")

	for i, source := range sources {
		if i >= len(urls) {
			break
		}

		// 分割单集
		parts := strings.Split(urls[i], "#")
		for j, part := range parts {
			eps := strings.Split(part, "$")
			if len(eps) >= 2 {
				episodes = append(episodes, model.Episode{
					EpisodeID: fmt.Sprintf("%d-%d", i, j),
					Name:      fmt.Sprintf("%s 第%d集", source, j+1),
					PlayURL:   eps[1],
				})
			}
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

// GetHomeData 获取首页数据
func (s *SearchService) GetHomeData() (*model.HomeData, error) {
	return &model.HomeData{
		Banner: []model.BannerItem{},
		Hot:    []model.SearchItem{},
		New:    []model.SearchItem{},
	}, nil
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
	parsed, err := url.Parse(targetURL)
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
