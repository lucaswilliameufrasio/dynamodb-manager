import 'package:flutter_test/flutter_test.dart';
import 'package:dynamodb_manager/src/models/dynamo_item_validation.dart';
import 'package:dynamodb_manager/src/models/dynamo_item.dart';

void main() {
  test('DynamoItem parses and formats one item from a scan page', () {
    final item = DynamoItem.fromDynamoJson('{"pk":"user-1","value":42}');

    expect(item.id, 'pk: user-1');
    expect(item.jsonContent, '{\n  "pk": "user-1",\n  "value": 42\n}');
  });

  group('parseNewDynamoItem', () {
    test('accepts an object containing both table keys', () {
      expect(
        parseNewDynamoItem(
          '{"pk":"user-1","sk":1,"name":"Ada"}',
          partitionKey: 'pk',
          sortKey: 'sk',
        ),
        {'pk': 'user-1', 'sk': 1, 'name': 'Ada'},
      );
    });

    test('rejects malformed JSON and non-object JSON', () {
      expect(
        () => parseNewDynamoItem('{', partitionKey: 'pk'),
        throwsFormatException,
      );
      expect(
        () => parseNewDynamoItem('[]', partitionKey: 'pk'),
        throwsFormatException,
      );
    });

    test('requires the partition and configured sort keys', () {
      expect(
        () => parseNewDynamoItem('{}', partitionKey: 'pk'),
        throwsA(
          isA<FormatException>().having(
            (e) => e.message,
            'message',
            contains('partition key'),
          ),
        ),
      );
      expect(
        () =>
            parseNewDynamoItem('{"pk":"x"}', partitionKey: 'pk', sortKey: 'sk'),
        throwsA(
          isA<FormatException>().having(
            (e) => e.message,
            'message',
            contains('sort key'),
          ),
        ),
      );
    });
  });
}
