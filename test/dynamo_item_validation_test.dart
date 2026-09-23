import 'package:flutter_test/flutter_test.dart';
import 'package:dynamodb_manager/src/models/dynamo_item_validation.dart';

void main() {
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
