// This executable benchmark reports measurements to stdout by design.
// ignore_for_file: avoid_print

import 'dart:convert';

import 'package:dynamodb_manager/src/models/dynamo_item.dart';

const _pageSize = 50;
const _pagesPerSample = 20;
const _sampleCount = 30;
const _warmupPages = 100;

void main() {
  final sourceItems = List.generate(_pageSize, _makeItemJson);
  var checksum = 0;

  int processPage() {
    var total = 0;
    for (final source in sourceItems) {
      final item = DynamoItem.fromDynamoJson(source);
      total += item.id.length + item.jsonContent.length;
    }
    return total;
  }

  for (var i = 0; i < _warmupPages; i++) {
    checksum += processPage();
  }

  final durations = <double>[];
  for (var sample = 0; sample < _sampleCount; sample++) {
    final stopwatch = Stopwatch()..start();
    var sampleChecksum = 0;
    for (var page = 0; page < _pagesPerSample; page++) {
      sampleChecksum += processPage();
    }
    stopwatch.stop();
    checksum += sampleChecksum;
    durations.add(stopwatch.elapsedMicroseconds / _pagesPerSample);
  }

  durations.sort();
  final medianUs = _percentile(durations, 0.5);
  final p95Us = _percentile(durations, 0.95);
  final itemsPerSecond = _pageSize * 1000000 / medianUs;

  print('DynamoItem page conversion baseline');
  print(
    'page_items=$_pageSize samples=$_sampleCount warmup_pages=$_warmupPages mode=AOT',
  );
  print('median_page_ms=${(medianUs / 1000).toStringAsFixed(3)}');
  print('p95_page_ms=${(p95Us / 1000).toStringAsFixed(3)}');
  print('median_items_per_second=${itemsPerSecond.toStringAsFixed(0)}');
  print('checksum=$checksum');
}

String _makeItemJson(int index) => jsonEncode({
  'pk': 'customer-${index.toString().padLeft(5, '0')}',
  'sk': 'order-${(index * 19).toString().padLeft(7, '0')}',
  'status': index.isEven ? 'pending' : 'complete',
  'amount': index * 12.75,
  'attempts': index % 6,
  'enabled': index.isEven,
  'tags': ['invoice', 'region-${index % 8}', 'batch'],
  'metadata': {
    'source': 'benchmark',
    'requestId': 'req-${index.toString().padLeft(8, '0')}',
    'attributes': {'tier': 'standard', 'retryable': index % 3 == 0},
  },
  'history': List.generate(
    4,
    (event) => {
      'state': event.isEven ? 'created' : 'updated',
      'timestamp': '2026-01-01T00:${event.toString().padLeft(2, '0')}:00Z',
      'actor': 'worker-$event',
    },
  ),
});

double _percentile(List<double> sorted, double percentile) {
  final index = ((sorted.length - 1) * percentile).ceil();
  return sorted[index];
}
