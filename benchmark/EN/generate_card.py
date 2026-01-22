#!/usr/bin/env python3
"""
为英文数据集生成 card.csv 文件
从 keyword_index.json 中提取问题，关键词就是问题
"""

import json
import csv
from pathlib import Path

def generate_card_csv():
    json_path = Path(__file__).parent / "processed" / "keyword_index.json"
    csv_path = Path(__file__).parent / "card.csv"
    
    if not json_path.exists():
        print(f"❌ JSON 文件不存在: {json_path}")
        return
    
    try:
        with open(json_path, 'r', encoding='utf-8') as f:
            keyword_index = json.load(f)
        
        print(f"📖 从 {json_path} 中读取 {len(keyword_index)} 个关键词")
        
        # 生成 CSV
        with open(csv_path, 'w', newline='', encoding='utf-8') as f:
            writer = csv.writer(f)
            # 写入表头
            writer.writerow(['keyword', 'question'])
            
            # 写入每个关键词和对应的问题
            for idx, keyword in enumerate(sorted(keyword_index.keys()), 1):
                writer.writerow([keyword, keyword])
        
        print(f"✅ 已生成 {csv_path}，包含 {len(keyword_index)} 个测试用例")
        print(f"📋 示例:")
        for keyword in sorted(keyword_index.keys())[:5]:
            files = keyword_index[keyword]
            print(f"   '{keyword}' -> {files}")
        
    except json.JSONDecodeError as e:
        print(f"❌ JSON 解析错误: {e}")
    except Exception as e:
        print(f"❌ 错误: {e}")

if __name__ == "__main__":
    generate_card_csv()
